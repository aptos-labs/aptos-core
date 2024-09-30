
<a id="0x1_permissioned_signer"></a>

# Module `0x1::permissioned_signer`

A _permissioned signer_ consists of a pair of the original signer and a generated
signer which is used store information about associated permissions.

A permissioned signer behaves compatible with the original signer as it comes to <code><b>move_to</b></code>, <code>address_of</code>, and
existing basic signer functionality. However, the permissions can be queried to assert additional
restrictions on the use of the signer.

A client which is interested in restricting access granted via a signer can create a permissioned signer
and pass on to other existing code without changes to existing APIs. Core functions in the framework, for
example account functions, can then assert availability of permissions, effectively restricting
existing code in a compatible way.

After introducing the core functionality, examples are provided for withdraw limit on accounts, and
for blind signing.


-  [Resource `GrantedPermissionHandles`](#0x1_permissioned_signer_GrantedPermissionHandles)
-  [Struct `PermissionedHandle`](#0x1_permissioned_signer_PermissionedHandle)
-  [Struct `StorablePermissionedHandle`](#0x1_permissioned_signer_StorablePermissionedHandle)
-  [Resource `PermStorage`](#0x1_permissioned_signer_PermStorage)
-  [Struct `Permission`](#0x1_permissioned_signer_Permission)
-  [Constants](#@Constants_0)
-  [Function `create_permissioned_handle`](#0x1_permissioned_signer_create_permissioned_handle)
-  [Function `create_storable_permissioned_handle`](#0x1_permissioned_signer_create_storable_permissioned_handle)
-  [Function `destroy_permissioned_handle`](#0x1_permissioned_signer_destroy_permissioned_handle)
-  [Function `destroy_storable_permissioned_handle`](#0x1_permissioned_signer_destroy_storable_permissioned_handle)
-  [Function `signer_from_permissioned`](#0x1_permissioned_signer_signer_from_permissioned)
-  [Function `signer_from_storable_permissioned`](#0x1_permissioned_signer_signer_from_storable_permissioned)
-  [Function `revoke_permission_handle`](#0x1_permissioned_signer_revoke_permission_handle)
-  [Function `permission_address`](#0x1_permissioned_signer_permission_address)
-  [Function `assert_master_signer`](#0x1_permissioned_signer_assert_master_signer)
-  [Function `authorize`](#0x1_permissioned_signer_authorize)
-  [Function `check_permission_exists`](#0x1_permissioned_signer_check_permission_exists)
-  [Function `check_permission_capacity_above`](#0x1_permissioned_signer_check_permission_capacity_above)
-  [Function `check_permission_consume`](#0x1_permissioned_signer_check_permission_consume)
-  [Function `capacity`](#0x1_permissioned_signer_capacity)
-  [Function `revoke_permission`](#0x1_permissioned_signer_revoke_permission)
-  [Function `extract_permission`](#0x1_permissioned_signer_extract_permission)
-  [Function `get_key`](#0x1_permissioned_signer_get_key)
-  [Function `address_of`](#0x1_permissioned_signer_address_of)
-  [Function `consume_permission`](#0x1_permissioned_signer_consume_permission)
-  [Function `store_permission`](#0x1_permissioned_signer_store_permission)
-  [Function `is_permissioned_signer`](#0x1_permissioned_signer_is_permissioned_signer)
-  [Function `permission_signer`](#0x1_permissioned_signer_permission_signer)
-  [Function `signer_from_permissioned_impl`](#0x1_permissioned_signer_signer_from_permissioned_impl)
-  [Specification](#@Specification_1)
    -  [Function `is_permissioned_signer`](#@Specification_1_is_permissioned_signer)
    -  [Function `permission_signer`](#@Specification_1_permission_signer)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_permissioned_signer_GrantedPermissionHandles"></a>

## Resource `GrantedPermissionHandles`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>revoked_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_permissioned_signer_PermissionedHandle"></a>

## Struct `PermissionedHandle`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>master_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>permission_addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_permissioned_signer_StorablePermissionedHandle"></a>

## Struct `StorablePermissionedHandle`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>master_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>permission_addr: <b>address</b></code>
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

<a id="0x1_permissioned_signer_PermStorage"></a>

## Resource `PermStorage`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>perms: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a>, u256&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_permissioned_signer_Permission"></a>

## Struct `Permission`



<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;K&gt;
</code></pre>



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
<code>capacity: u256</code>
</dt>
<dd>

</dd>
</dl>


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



<a id="0x1_permissioned_signer_create_permissioned_handle"></a>

## Function `create_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permission_addr = generate_auid_address();
    <b>let</b> master_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a> {
        master_addr,
        permission_addr,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_create_storable_permissioned_handle"></a>

## Function `create_storable_permissioned_handle`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permission_addr = generate_auid_address();
    <b>let</b> master_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_addr)) {
        <b>move_to</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master, <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
            active_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            revoked_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        });
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_addr).active_handles,
        permission_addr
    );

    // Do we need <b>to</b> <b>move</b> sth similar <b>to</b> ObjectCore <b>to</b> register this <b>address</b> <b>as</b> permission <b>address</b>?
    <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> {
        master_addr,
        permission_addr,
        expiration_time,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_destroy_permissioned_handle"></a>

## Function `destroy_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a> { master_addr: _, permission_addr } = p;
    <b>if</b>(<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_addr)) {
        <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> { perms } = <b>move_from</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_addr);
        <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_destroy">smart_table::destroy</a>(perms);
    };
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_destroy_storable_permissioned_handle"></a>

## Function `destroy_storable_permissioned_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">destroy_storable_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_storable_permissioned_handle">destroy_storable_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> { master_addr, permission_addr, expiration_time: _ } = p;
    <b>if</b>(<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_addr)) {
        <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> { perms } = <b>move_from</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_addr);
        <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_destroy">smart_table::destroy</a>(perms);
    };
    <b>let</b> granted_permissions = <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_addr);
    <b>let</b> (found, idx) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&granted_permissions.active_handles, &permission_addr);
    <b>if</b>(found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> granted_permissions.active_handles, idx);
    };
    <b>let</b> (found, idx) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&granted_permissions.revoked_handles, &permission_addr);
    <b>if</b>(found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> granted_permissions.revoked_handles, idx);
    };
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned"></a>

## Function `signer_from_permissioned`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned">signer_from_permissioned</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned">signer_from_permissioned</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_impl">signer_from_permissioned_impl</a>(p.master_addr, p.permission_addr)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_storable_permissioned"></a>

## Function `signer_from_storable_permissioned`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned">signer_from_storable_permissioned</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned">signer_from_storable_permissioned</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <b>assert</b>!(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; p.expiration_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>));
    <b>assert</b>!(
        !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(
            &<b>borrow_global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(p.master_addr).revoked_handles,
            &p.permission_addr
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_impl">signer_from_permissioned_impl</a>(p.master_addr, p.permission_addr)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission_handle"></a>

## Function `revoke_permission_handle`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_handle">revoke_permission_handle</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permission_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_handle">revoke_permission_handle</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permission_addr: <b>address</b>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <b>assert</b>!(!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> master_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s);
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_addr)) {
        <b>return</b>
    };
    <b>let</b> granted_permissions = <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_addr);
    <b>if</b>(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&granted_permissions.revoked_handles, &permission_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> granted_permissions.revoked_handles, permission_addr)
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_permission_address"></a>

## Function `permission_address`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>): <b>address</b> {
    p.permission_addr
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_assert_master_signer"></a>

## Function `assert_master_signer`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_authorize"></a>

## Function `authorize`

=====================================================================================================
Permission Management

Authorizes <code>permissioned</code> with the given permission. This requires to have access to the <code>master</code>
signer.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">authorize</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize">authorize</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    capacity: u256,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned) &&
        !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(master) &&
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master) == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>)
    );
    <b>let</b> permission_signer = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(permissioned);
    <b>let</b> permission_signer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&permission_signer);
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_signer_addr)) {
        <b>move_to</b>(&permission_signer, <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> { perms: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>()});
    };
    <b>let</b> perms = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b>(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(perms, key)) {
        <b>let</b> entry = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(perms, key);
        *entry = *entry + capacity;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(perms, key, capacity);
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_exists"></a>

## Function `check_permission_exists`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
        <b>return</b> <b>true</b>
    };
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(s));
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr)) {
        <b>return</b> <b>false</b>
    };
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms, <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm))
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_capacity_above"></a>

## Function `check_permission_capacity_above`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    threshold: u256,
    perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
        <b>return</b> <b>true</b>
    };
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(s));
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>let</b> storage = &<b>borrow_global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms;
    <b>if</b>(!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(storage, key)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> perm = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(storage, key);
    <b>if</b>(*perm &gt; threshold) {
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_consume"></a>

## Function `check_permission_consume`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    threshold: u256,
    perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
        <b>return</b> <b>true</b>
    };
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(s));
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>let</b> storage = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms;
    <b>if</b>(!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(storage, key)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> perm = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(storage, key);
    <b>if</b>(*perm &gt;= threshold) {
        *perm = *perm - threshold;
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_capacity"></a>

## Function `capacity`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): Option&lt;u256&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>assert</b>!(<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER">ENOT_PERMISSIONED_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(s));
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> perm_storage = &<b>borrow_global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b>(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(perm_storage, key)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<b>borrow_global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms, key))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission"></a>

## Function `revoke_permission`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>if</b>(!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)) {
        // Master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> no permissions associated <b>with</b> it.
        <b>return</b>
    };
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(permissioned));
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr)) {
        <b>return</b>
    };
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(addr).perms, <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm));
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_extract_permission"></a>

## Function `extract_permission`

Another flavor of api to extract and store permissions


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_permission">extract_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, weight: u256, perm: PermKey): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_extract_permission">extract_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    weight: u256,
    perm: PermKey
): <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>assert</b>!(<a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>(s, weight, perm), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_EXTRACT_PERMISSION">ECANNOT_EXTRACT_PERMISSION</a>));
    <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a> {
        owner_address: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s),
        key: perm,
        capacity: weight,
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_get_key"></a>

## Function `get_key`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_get_key">get_key</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;): &PermKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_get_key">get_key</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;): &PermKey {
    &perm.key
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_address_of"></a>

## Function `address_of`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>&lt;PermKey&gt;(perm: &<a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;): <b>address</b> {
    perm.owner_address
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_consume_permission"></a>

## Function `consume_permission`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">consume_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;, weight: u256, perm_key: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_consume_permission">consume_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    perm: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;,
    weight: u256,
    perm_key: PermKey
): bool {
    <b>if</b>(perm.key != perm_key) {
        <b>return</b> <b>false</b>
    };
    <b>if</b>(perm.capacity &gt;= weight) {
        perm.capacity = perm.capacity - weight;
        <b>return</b> <b>true</b>
    } <b>else</b> {
        <b>return</b> <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_store_permission"></a>

## Function `store_permission`



<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_store_permission">store_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">permissioned_signer::Permission</a>&lt;PermKey&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_store_permission">store_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a>&lt;PermKey&gt;
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> {
    <b>assert</b>!(<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER">ENOT_PERMISSIONED_SIGNER</a>));
    <b>let</b> <a href="permissioned_signer.md#0x1_permissioned_signer_Permission">Permission</a> { key, capacity, owner_address } = perm;

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s) == owner_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_MISMATCH">E_PERMISSION_MISMATCH</a>));

    <b>let</b> permission_signer = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(s);
    <b>let</b> permission_signer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&permission_signer);
    <b>if</b>(!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_signer_addr)) {
        <b>move_to</b>(&permission_signer, <a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a> { perms: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>()});
    };
    <b>let</b> perms = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermStorage">PermStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(key);
    <b>if</b>(<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(perms, key)) {
        <b>let</b> entry = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut">smart_table::borrow_mut</a>(perms, key);
        *entry = *entry + capacity;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(perms, key, capacity)
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_is_permissioned_signer"></a>

## Function `is_permissioned_signer`

Creates a permissioned signer from an existing universal signer. The function aborts if the
given signer is already a permissioned signer.

The implementation of this function requires to extend the value representation for signers in the VM.

Check whether this is a permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



</details>

<a id="0x1_permissioned_signer_permission_signer"></a>

## Function `permission_signer`

Return the signer used for storing permissions. Aborts if not a permissioned signer.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned_impl"></a>

## Function `signer_from_permissioned_impl`


invariants:
signer::address_of(master) == signer::address_of(signer_from_permissioned(create_permissioned_handle(master))),


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_impl">signer_from_permissioned_impl</a>(master_addr: <b>address</b>, permission_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_impl">signer_from_permissioned_impl</a>(master_addr: <b>address</b>, permission_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_permissioned_signer_spec_is_permissioned_signer"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



<a id="@Specification_1_is_permissioned_signer"></a>

### Function `is_permissioned_signer`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s);
</code></pre>




<a id="0x1_permissioned_signer_spec_permission_signer"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_signer">spec_permission_signer</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



<a id="@Specification_1_permission_signer"></a>

### Function `permission_signer`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_signer">permission_signer</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_signer">spec_permission_signer</a>(permissioned);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
