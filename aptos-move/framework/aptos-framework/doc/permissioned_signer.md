
<a id="0x1_permissioned_signer"></a>

# Module `0x1::permissioned_signer`

A _permissioned signer_ consists of a pair of the original signer and a generated
address which is used to store information about associated permissions.

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


-  [Struct `RevokePermissionHandlePermission`](#0x1_permissioned_signer_RevokePermissionHandlePermission)
-  [Resource `GrantedPermissionHandles`](#0x1_permissioned_signer_GrantedPermissionHandles)
-  [Enum `PermissionedHandle`](#0x1_permissioned_signer_PermissionedHandle)
-  [Enum `StorablePermissionedHandle`](#0x1_permissioned_signer_StorablePermissionedHandle)
-  [Enum Resource `PermissionStorage`](#0x1_permissioned_signer_PermissionStorage)
-  [Enum `StoredPermission`](#0x1_permissioned_signer_StoredPermission)
-  [Constants](#@Constants_0)
-  [Function `create_permissioned_handle`](#0x1_permissioned_signer_create_permissioned_handle)
-  [Function `destroy_permissioned_handle`](#0x1_permissioned_signer_destroy_permissioned_handle)
-  [Function `signer_from_permissioned_handle`](#0x1_permissioned_signer_signer_from_permissioned_handle)
-  [Function `is_permissioned_signer`](#0x1_permissioned_signer_is_permissioned_signer)
-  [Function `grant_revoke_permission`](#0x1_permissioned_signer_grant_revoke_permission)
-  [Function `revoke_permission_storage_address`](#0x1_permissioned_signer_revoke_permission_storage_address)
-  [Function `revoke_all_handles`](#0x1_permissioned_signer_revoke_all_handles)
-  [Function `initialize_permission_address`](#0x1_permissioned_signer_initialize_permission_address)
-  [Function `create_storable_permissioned_handle`](#0x1_permissioned_signer_create_storable_permissioned_handle)
-  [Function `destroy_storable_permissioned_handle`](#0x1_permissioned_signer_destroy_storable_permissioned_handle)
-  [Function `destroy_permissions_storage_address`](#0x1_permissioned_signer_destroy_permissions_storage_address)
-  [Function `signer_from_storable_permissioned_handle`](#0x1_permissioned_signer_signer_from_storable_permissioned_handle)
-  [Function `permissions_storage_address`](#0x1_permissioned_signer_permissions_storage_address)
-  [Function `assert_master_signer`](#0x1_permissioned_signer_assert_master_signer)
-  [Function `is_above`](#0x1_permissioned_signer_is_above)
-  [Function `consume_capacity`](#0x1_permissioned_signer_consume_capacity)
-  [Function `increase_capacity`](#0x1_permissioned_signer_increase_capacity)
-  [Function `merge`](#0x1_permissioned_signer_merge)
-  [Function `map_or`](#0x1_permissioned_signer_map_or)
-  [Function `insert_or`](#0x1_permissioned_signer_insert_or)
-  [Function `authorize_increase`](#0x1_permissioned_signer_authorize_increase)
-  [Function `authorize_unlimited`](#0x1_permissioned_signer_authorize_unlimited)
-  [Function `grant_unlimited_with_permissioned_signer`](#0x1_permissioned_signer_grant_unlimited_with_permissioned_signer)
-  [Function `increase_limit`](#0x1_permissioned_signer_increase_limit)
-  [Function `check_permission_exists`](#0x1_permissioned_signer_check_permission_exists)
-  [Function `check_permission_capacity_above`](#0x1_permissioned_signer_check_permission_capacity_above)
-  [Function `check_permission_consume`](#0x1_permissioned_signer_check_permission_consume)
-  [Function `capacity`](#0x1_permissioned_signer_capacity)
-  [Function `revoke_permission`](#0x1_permissioned_signer_revoke_permission)
-  [Function `address_of`](#0x1_permissioned_signer_address_of)
-  [Function `borrow_address`](#0x1_permissioned_signer_borrow_address)
-  [Function `is_permissioned_signer_impl`](#0x1_permissioned_signer_is_permissioned_signer_impl)
-  [Function `permission_address`](#0x1_permissioned_signer_permission_address)
-  [Function `signer_from_permissioned_handle_impl`](#0x1_permissioned_signer_signer_from_permissioned_handle_impl)
-  [Specification](#@Specification_1)
    -  [Function `create_permissioned_handle`](#@Specification_1_create_permissioned_handle)
    -  [Function `destroy_permissioned_handle`](#@Specification_1_destroy_permissioned_handle)
    -  [Function `is_permissioned_signer`](#@Specification_1_is_permissioned_signer)
    -  [Function `revoke_permission_storage_address`](#@Specification_1_revoke_permission_storage_address)
    -  [Function `create_storable_permissioned_handle`](#@Specification_1_create_storable_permissioned_handle)
    -  [Function `destroy_storable_permissioned_handle`](#@Specification_1_destroy_storable_permissioned_handle)
    -  [Function `authorize_increase`](#@Specification_1_authorize_increase)
    -  [Function `check_permission_exists`](#@Specification_1_check_permission_exists)
    -  [Function `check_permission_capacity_above`](#@Specification_1_check_permission_capacity_above)
    -  [Function `check_permission_consume`](#@Specification_1_check_permission_consume)
    -  [Function `capacity`](#@Specification_1_capacity)
    -  [Function `is_permissioned_signer_impl`](#@Specification_1_is_permissioned_signer_impl)
    -  [Function `permission_address`](#@Specification_1_permission_address)
    -  [Function `signer_from_permissioned_handle_impl`](#@Specification_1_signer_from_permissioned_handle_impl)


<pre><code><b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_permissioned_signer_RevokePermissionHandlePermission"></a>

## Struct `RevokePermissionHandlePermission`

If a permissioned signer has this permission, it would be able to revoke other granted
permission handles in the same signer.


<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_RevokePermissionHandlePermission">RevokePermissionHandlePermission</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_permissioned_signer_GrantedPermissionHandles"></a>

## Resource `GrantedPermissionHandles`

Stores the list of granted permission handles for a given account.


<pre><code><b>struct</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>active_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>
 Each address refers to a <code>permissions_storage_addr</code> that stores the <code><a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a></code>.
</dd>
</dl>


</details>

<a id="0x1_permissioned_signer_PermissionedHandle"></a>

## Enum `PermissionedHandle`

A ephermeral permission handle that can be used to generate a permissioned signer with permission
configuration stored within.


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
 Address of the signer that creates this handle.
</dd>
<dt>
<code>permissions_storage_addr: <b>address</b></code>
</dt>
<dd>
 Address that stores <code><a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a></code>.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_StorablePermissionedHandle"></a>

## Enum `StorablePermissionedHandle`

A permission handle that can be used to generate a permissioned signer.

This handle is storable and thus should be treated very carefully as it serves similar functionality
as signer delegation.


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
 Address of the signer that creates this handle.
</dd>
<dt>
<code>permissions_storage_addr: <b>address</b></code>
</dt>
<dd>
 Address that stores <code><a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a></code>.
</dd>
<dt>
<code>expiration_time: u64</code>
</dt>
<dd>
 Permissioned signer can no longer be generated from this handle after <code>expiration_time</code>.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_PermissionStorage"></a>

## Enum Resource `PermissionStorage`

The actual permission configuration stored on-chain.

The address that holds <code><a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a></code> will be generated freshly every time a permission
handle gets created.


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
<code>perms: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>&gt;</code>
</dt>
<dd>
 A hetherogenous map from <code>Permission</code> structs defined by each different modules to
 its permission capacity.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_permissioned_signer_StoredPermission"></a>

## Enum `StoredPermission`

Types of permission capacity stored on chain.


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

Trying to grant permission using non-master signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER"></a>

Access permission information from a master signer.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_PERMISSIONED_SIGNER">ENOT_PERMISSIONED_SIGNER</a>: u64 = 3;
</code></pre>



<a id="0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED"></a>

Permissioned signer feature is not activated.


<pre><code><b>const</b> <a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>: u64 = 9;
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


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );

    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permissions_storage_addr = generate_auid_address();
    <b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <a href="permissioned_signer.md#0x1_permissioned_signer_initialize_permission_address">initialize_permission_address</a>(permissions_storage_addr);

    PermissionedHandle::V1 { master_account_addr, permissions_storage_addr }
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
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <b>let</b> PermissionedHandle::V1 { master_account_addr: _, permissions_storage_addr } =
        p;
    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr);
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned_handle"></a>

## Function `signer_from_permissioned_handle`

Generate the permissioned signer based on the ephermeral permission handle.

This signer can be used as a regular signer for other smart contracts. However when such
signer interacts with various framework functions, it would subject to permission checks
and would abort if check fails.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle">signer_from_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle">signer_from_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">PermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
        p.master_account_addr, p.permissions_storage_addr
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_is_permissioned_signer"></a>

## Function `is_permissioned_signer`

Returns true if <code>s</code> is a permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool {
    // When the permissioned <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> is disabled, no one is able <b>to</b> construct a permissioned
    // <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>. Thus we should <b>return</b> <b>false</b> here, <b>as</b> other on chain permission checks will
    // depend on this checks.
    <b>if</b>(!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>()) {
        <b>return</b> <b>false</b>;
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer_impl">is_permissioned_signer_impl</a>(s)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_grant_revoke_permission"></a>

## Function `grant_revoke_permission`

Grant the permissioned signer the permission to revoke granted permission handles under
its address.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_grant_revoke_permission">grant_revoke_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_grant_revoke_permission">grant_revoke_permission</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">authorize_unlimited</a>(master, permissioned, <a href="permissioned_signer.md#0x1_permissioned_signer_RevokePermissionHandlePermission">RevokePermissionHandlePermission</a> {});
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission_storage_address"></a>

## Function `revoke_permission_storage_address`

Revoke a specific storable permission handle immediately. This will disallow owner of
the storable permission handle to derive signer from it anymore.


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>(s, <a href="permissioned_signer.md#0x1_permissioned_signer_RevokePermissionHandlePermission">RevokePermissionHandlePermission</a> {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
    );
    <b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s);

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>),
    );
    <b>let</b> active_handles = &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>[master_account_addr].active_handles;
    <b>let</b> (found, idx) = active_handles.index_of(&permissions_storage_addr);

    // The <b>address</b> <b>has</b> <b>to</b> be in the activated list in the master <a href="account.md#0x1_account">account</a> <b>address</b>.
    <b>assert</b>!(found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>));
    active_handles.swap_remove(idx);
    <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr);
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_all_handles"></a>

## Function `revoke_all_handles`

Revoke all storable permission handle of the signer immediately.


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_all_handles">revoke_all_handles</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_all_handles">revoke_all_handles</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>, <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>(s, <a href="permissioned_signer.md#0x1_permissioned_signer_RevokePermissionHandlePermission">RevokePermissionHandlePermission</a> {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
    );
    <b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(s);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr)) { <b>return</b> };

    <b>let</b> granted_permissions =
        <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
    <b>let</b> delete_list = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_trim_reverse">vector::trim_reverse</a>(
        &<b>mut</b> granted_permissions.active_handles, 0
    );
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_destroy">vector::destroy</a>(
        delete_list,
        |<b>address</b>| {
            <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(<b>address</b>);
        }
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_initialize_permission_address"></a>

## Function `initialize_permission_address`

initialize permission storage by putting an empty storage under the address.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_initialize_permission_address">initialize_permission_address</a>(permissions_storage_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_initialize_permission_address">initialize_permission_address</a>(permissions_storage_addr: <b>address</b>) {
    <b>move_to</b>(
        &<a href="create_signer.md#0x1_create_signer">create_signer</a>(permissions_storage_addr),
        // Each key is ~100bytes, the value is 12 bytes.
        PermissionStorage::V1 { perms: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">big_ordered_map::new_with_config</a>(40, 35, <b>false</b>) }
    );
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_create_storable_permissioned_handle"></a>

## Function `create_storable_permissioned_handle`

Create an storable permission handle based on the master signer.

This handle can be used to derive a signer that can be stored by a smart contract.
This is as dangerous as key delegation, thus it remains public(package) for now.

The caller should check if <code>expiration_time</code> is not too far in the future.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, expiration_time: u64
): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a> <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );

    <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(master);
    <b>let</b> permissions_storage_addr = generate_auid_address();
    <b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);

    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>)
    );

    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr)) {
        <b>move_to</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(
            master, <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a> { active_handles: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>() }
        );
    };

    <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>[master_account_addr]
        .active_handles.push_back(permissions_storage_addr);

    <a href="permissioned_signer.md#0x1_permissioned_signer_initialize_permission_address">initialize_permission_address</a>(permissions_storage_addr);

    StorablePermissionedHandle::V1 {
        master_account_addr,
        permissions_storage_addr,
        expiration_time
    }
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
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>),
    );
    <b>let</b> active_handles = &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>[master_account_addr].active_handles;

    <b>let</b> (found, idx) = active_handles.index_of(&permissions_storage_addr);

    // Removing the <b>address</b> from the active handle list <b>if</b> it's still active.
    <b>if</b>(found) {
        active_handles.swap_remove(idx);
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


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissions_storage_address">destroy_permissions_storage_address</a>(permissions_storage_addr: <b>address</b>) {
    <b>if</b> (<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr)) {
        <b>let</b> PermissionStorage::V1 { perms } =
            <b>move_from</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
        <a href="big_ordered_map.md#0x1_big_ordered_map_destroy">big_ordered_map::destroy</a>(
            perms,
            |_dv| {},
        );
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_storable_permissioned_handle"></a>

## Function `signer_from_storable_permissioned_handle`

Generate the permissioned signer based on the storable permission handle.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">signer_from_storable_permissioned_handle</a>(p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_storable_permissioned_handle">signer_from_storable_permissioned_handle</a>(
    p: &<a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">StorablePermissionedHandle</a>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_permissioned_signer_enabled">features::is_permissioned_signer_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
    );
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt; p.expiration_time,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_EXPIRED">E_PERMISSION_EXPIRED</a>)
    );
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(p.permissions_storage_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_PERMISSION_REVOKED">E_PERMISSION_REVOKED</a>)
    );
    <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
        p.master_account_addr, p.permissions_storage_addr
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_assert_master_signer">assert_master_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>)
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
        StoredPermission::Capacity(capacity) =&gt; *capacity &gt;= threshold,
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


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>&lt;PermKey: <b>copy</b>, drop, store, T&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey, mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>|T, default: T): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>&lt;PermKey: <b>copy</b> + drop + store, T&gt;(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey,
    mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>| T,
    default: T,
): T {
    <b>let</b> permission_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>)
    );
    <b>let</b> perms =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (<a href="big_ordered_map.md#0x1_big_ordered_map_contains">big_ordered_map::contains</a>(perms, &key)) {
        <b>let</b> value = perms.remove(&key);
        <b>let</b> return_ = mutate(&<b>mut</b> value);
        perms.add(key, value);
        return_
    } <b>else</b> {
        default
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_insert_or"></a>

## Function `insert_or`



<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey, mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>|, default: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">permissioned_signer::StoredPermission</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_insert_or">insert_or</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey,
    mutate: |&<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>|,
    default: <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>,
) {
    <b>let</b> permission_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_E_NOT_ACTIVE">E_NOT_ACTIVE</a>)
    );
    <b>let</b> perms =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permission_signer_addr).perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (perms.contains(&key)) {
        <b>let</b> value = perms.remove(&key);
        mutate(&<b>mut</b> value);
        perms.add(key, value);
    } <b>else</b> {
        perms.add(key, default);
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_authorize_increase"></a>

## Function `authorize_increase`

Authorizes <code>permissioned</code> with a given capacity and increment the existing capacity if present.

Consumption using <code>check_permission_consume</code> will deduct the capacity.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_increase">authorize_increase</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_increase">authorize_increase</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    capacity: u256,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)
            && !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(master)
            && <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master) == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>)
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">authorize_unlimited</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">authorize_unlimited</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)
            && !<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(master)
            && <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master) == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_ECANNOT_AUTHORIZE">ECANNOT_AUTHORIZE</a>)
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

<a id="0x1_permissioned_signer_grant_unlimited_with_permissioned_signer"></a>

## Function `grant_unlimited_with_permissioned_signer`

Grant an unlimited permission to a permissioned signer **without** master signer's approvoal.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_grant_unlimited_with_permissioned_signer">grant_unlimited_with_permissioned_signer</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_grant_unlimited_with_permissioned_signer">grant_unlimited_with_permissioned_signer</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b>(!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)) {
        <b>return</b>;
    };
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

<a id="0x1_permissioned_signer_increase_limit"></a>

## Function `increase_limit`

Increase the <code>capacity</code> of a permissioned signer **without** master signer's approvoal.

The caller of the module will need to make sure the witness type <code>PermKey</code> can only be
constructed within its own module, otherwise attackers can refill the permission for itself
to bypass the checks.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_increase_limit">increase_limit</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_increase_limit">increase_limit</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    capacity: u256,
    perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b>(!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)) {
        <b>return</b>;
    };
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

<a id="0x1_permissioned_signer_check_permission_exists"></a>

## Function `check_permission_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    // 0 capacity permissions will be treated <b>as</b> non-existant.
    <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>(s, 1, perm)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_check_permission_capacity_above"></a>

## Function `check_permission_capacity_above`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
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



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey
): bool <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        // master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> all permissions
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



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey
): Option&lt;u256&gt; <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_U256_MAX">U256_MAX</a>)
    };
    <a href="permissioned_signer.md#0x1_permissioned_signer_map_or">map_or</a>(
        s,
        perm,
        |stored_permission: &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_StoredPermission">StoredPermission</a>| {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(match (stored_permission) {
                StoredPermission::Capacity(capacity) =&gt; *capacity,
                StoredPermission::Unlimited =&gt; <a href="permissioned_signer.md#0x1_permissioned_signer_U256_MAX">U256_MAX</a>,
            })
        },
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    )
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_revoke_permission"></a>

## Function `revoke_permission`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission">revoke_permission</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey
) <b>acquires</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a> {
    <b>if</b> (!<a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(permissioned)) {
        // Master <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>has</b> no permissions associated <b>with</b> it.
        <b>return</b>
    };
    <b>let</b> addr = <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned);
    <b>if</b> (!<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(addr)) { <b>return</b> };
    <b>let</b> perm_storage = &<b>mut</b> <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>[addr].perms;
    <b>let</b> key = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(perm);
    <b>if</b> (perm_storage.contains(&key)) {
        perm_storage.remove(&key);
    }
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_address_of"></a>

## Function `address_of`

Unused function. Keeping it for compatibility purpose.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>(_s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">address_of</a>(_s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b> {
    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_borrow_address"></a>

## Function `borrow_address`

Unused function. Keeping it for compatibility purpose.


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_borrow_address">borrow_address</a>(_s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_borrow_address">borrow_address</a>(_s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<b>address</b> {
    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="permissioned_signer.md#0x1_permissioned_signer_EPERMISSION_SIGNER_DISABLED">EPERMISSION_SIGNER_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_permissioned_signer_is_permissioned_signer_impl"></a>

## Function `is_permissioned_signer_impl`


Check whether this is a permissioned signer.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer_impl">is_permissioned_signer_impl</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer_impl">is_permissioned_signer_impl</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



</details>

<a id="0x1_permissioned_signer_permission_address"></a>

## Function `permission_address`

Return the address used for storing permissions. Aborts if not a permissioned signer.


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>;
</code></pre>



</details>

<a id="0x1_permissioned_signer_signer_from_permissioned_handle_impl"></a>

## Function `signer_from_permissioned_handle_impl`

Creates a permissioned signer from an existing universal signer. The function aborts if the
given signer is already a permissioned signer.

The implementation of this function requires to extend the value representation for signers in the VM.
invariants:
signer::address_of(master) == signer::address_of(signer_from_permissioned_handle(create_permissioned_handle(master))),


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(
    master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>axiom</b> <b>forall</b> a: <a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>:
    (
        <b>forall</b> i in 0..len(a.active_handles):
            <b>forall</b> j in 0..len(a.active_handles):
                i != j ==&gt;
                    a.active_handles[i] != a.active_handles[j]
    );
</code></pre>




<a id="0x1_permissioned_signer_spec_is_permissioned_signer_impl"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer_impl">spec_is_permissioned_signer_impl</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool;
</code></pre>



<a id="@Specification_1_create_permissioned_handle"></a>

### Function `create_permissioned_handle`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_permissioned_handle">create_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>let</b> permissions_storage_addr = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
<b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>ensures</b> result.master_account_addr == master_account_addr;
<b>ensures</b> result.permissions_storage_addr == permissions_storage_addr;
</code></pre>



<a id="@Specification_1_destroy_permissioned_handle"></a>

### Function `destroy_permissioned_handle`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_destroy_permissioned_handle">destroy_permissioned_handle</a>(p: <a href="permissioned_signer.md#0x1_permissioned_signer_PermissionedHandle">permissioned_signer::PermissionedHandle</a>)
</code></pre>




<pre><code><b>ensures</b> !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(p.permissions_storage_addr);
</code></pre>



<a id="@Specification_1_is_permissioned_signer"></a>

### Function `is_permissioned_signer`


<pre><code><b>public</b> <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s);
</code></pre>




<a id="0x1_permissioned_signer_spec_permission_address"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>;
</code></pre>



<a id="@Specification_1_revoke_permission_storage_address"></a>

### Function `revoke_permission_storage_address`


<pre><code><b>public</b> entry <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_revoke_permission_storage_address">revoke_permission_storage_address</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissions_storage_addr: <b>address</b>)
</code></pre>




<a id="@Specification_1_create_storable_permissioned_handle"></a>

### Function `create_storable_permissioned_handle`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_create_storable_permissioned_handle">create_storable_permissioned_handle</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, expiration_time: u64): <a href="permissioned_signer.md#0x1_permissioned_signer_StorablePermissionedHandle">permissioned_signer::StorablePermissionedHandle</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>let</b> permissions_storage_addr = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissions_storage_addr);
<b>let</b> master_account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
<b>ensures</b> result.master_account_addr == master_account_addr;
<b>ensures</b> result.permissions_storage_addr == permissions_storage_addr;
<b>ensures</b> result.expiration_time == expiration_time;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(
    <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr).active_handles,
    permissions_storage_addr
);
<b>ensures</b> <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_GrantedPermissionHandles">GrantedPermissionHandles</a>&gt;(master_account_addr);
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



<a id="@Specification_1_authorize_increase"></a>

### Function `authorize_increase`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_increase">authorize_increase</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, capacity: u256, perm: PermKey)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(permissioned);
<b>aborts_if</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(master);
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned) != <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(master);
<b>ensures</b> <b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(
    <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(permissioned)
);
</code></pre>



<a id="@Specification_1_check_permission_exists"></a>

### Function `check_permission_exists`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">check_permission_exists</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(<a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s));
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_exists">spec_check_permission_exists</a>(s, perm);
</code></pre>




<a id="0x1_permissioned_signer_spec_check_permission_exists"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_exists">spec_check_permission_exists</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): bool;
</code></pre>



<a id="@Specification_1_check_permission_capacity_above"></a>

### Function `check_permission_capacity_above`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_capacity_above">check_permission_capacity_above</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(<a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s));
<b>let</b> permissioned_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
<b>ensures</b> !<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s) ==&gt; result == <b>true</b>;
<b>ensures</b> (
    <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s)
        && !<b>exists</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(permissioned_signer_addr)
) ==&gt; result == <b>false</b>;
</code></pre>



<a id="@Specification_1_check_permission_consume"></a>

### Function `check_permission_consume`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_consume">check_permission_consume</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>let</b> permissioned_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(<a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s));
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_consume">spec_check_permission_consume</a>(s, threshold, perm);
</code></pre>




<a id="0x1_permissioned_signer_spec_check_permission_consume"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_check_permission_consume">spec_check_permission_consume</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, threshold: u256, perm: PermKey): bool;
</code></pre>



<a id="@Specification_1_capacity"></a>

### Function `capacity`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_capacity">capacity</a>&lt;PermKey: <b>copy</b>, drop, store&gt;(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u256&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>let</b> permissioned_signer_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s);
<b>modifies</b> <b>global</b>&lt;<a href="permissioned_signer.md#0x1_permissioned_signer_PermissionStorage">PermissionStorage</a>&gt;(<a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(s));
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_capacity">spec_capacity</a>(s, perm);
</code></pre>




<a id="0x1_permissioned_signer_spec_capacity"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_capacity">spec_capacity</a>&lt;PermKey: <b>copy</b> + drop + store&gt;(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, perm: PermKey): Option&lt;u256&gt;;
</code></pre>



<a id="@Specification_1_is_permissioned_signer_impl"></a>

### Function `is_permissioned_signer_impl`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_is_permissioned_signer_impl">is_permissioned_signer_impl</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer_impl">spec_is_permissioned_signer_impl</a>(s);
</code></pre>




<a id="0x1_permissioned_signer_spec_is_permissioned_signer"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool {
   <b>use</b> std::features;
   <b>use</b> std::features::PERMISSIONED_SIGNER;
   <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(PERMISSIONED_SIGNER)) {
       <b>false</b>
   } <b>else</b> {
       <a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer_impl">spec_is_permissioned_signer_impl</a>(s)
   }
}
</code></pre>



<a id="@Specification_1_permission_address"></a>

### Function `permission_address`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_permission_address">permission_address</a>(permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract]!<a href="permissioned_signer.md#0x1_permissioned_signer_spec_is_permissioned_signer">spec_is_permissioned_signer</a>(permissioned);
<b>ensures</b> [abstract] result == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_permission_address">spec_permission_address</a>(permissioned);
</code></pre>




<a id="0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl"></a>


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl">spec_signer_from_permissioned_handle_impl</a>(
   master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



<a id="@Specification_1_signer_from_permissioned_handle_impl"></a>

### Function `signer_from_permissioned_handle_impl`


<pre><code><b>fun</b> <a href="permissioned_signer.md#0x1_permissioned_signer_signer_from_permissioned_handle_impl">signer_from_permissioned_handle_impl</a>(master_account_addr: <b>address</b>, permissions_storage_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result
    == <a href="permissioned_signer.md#0x1_permissioned_signer_spec_signer_from_permissioned_handle_impl">spec_signer_from_permissioned_handle_impl</a>(
        master_account_addr, permissions_storage_addr
    );
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
