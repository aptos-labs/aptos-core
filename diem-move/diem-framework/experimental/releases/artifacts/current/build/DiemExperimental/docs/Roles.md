
<a name="0x1_Roles"></a>

# Module `0x1::Roles`



-  [Resource `RoleId`](#0x1_Roles_RoleId)
-  [Constants](#@Constants_0)
-  [Function `grant_diem_root_role`](#0x1_Roles_grant_diem_root_role)
-  [Function `new_validator_role`](#0x1_Roles_new_validator_role)
-  [Function `new_validator_operator_role`](#0x1_Roles_new_validator_operator_role)
-  [Function `grant_role`](#0x1_Roles_grant_role)
-  [Function `has_role`](#0x1_Roles_has_role)
-  [Function `has_diem_root_role`](#0x1_Roles_has_diem_root_role)
-  [Function `has_validator_role`](#0x1_Roles_has_validator_role)
-  [Function `has_validator_operator_role`](#0x1_Roles_has_validator_operator_role)
-  [Function `get_role_id`](#0x1_Roles_get_role_id)
-  [Function `assert_diem_root`](#0x1_Roles_assert_diem_root)
-  [Function `assert_validator`](#0x1_Roles_assert_validator)
-  [Function `assert_validator_operator`](#0x1_Roles_assert_validator_operator)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
</code></pre>



<a name="0x1_Roles_RoleId"></a>

## Resource `RoleId`

The roleId contains the role id for the account. This is only moved
to an account as a top-level resource, and is otherwise immovable.


<pre><code><b>struct</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>role_id: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Roles_DIEM_ROOT_ROLE_ID"></a>



<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_DIEM_ROOT_ROLE_ID">DIEM_ROOT_ROLE_ID</a>: u64 = 0;
</code></pre>



<a name="0x1_Roles_EDIEM_ROOT"></a>

The signer didn't have the required Diem Root role


<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_EDIEM_ROOT">EDIEM_ROOT</a>: u64 = 1;
</code></pre>



<a name="0x1_Roles_EROLE_ID"></a>

A <code><a href="Roles.md#0x1_Roles_RoleId">RoleId</a></code> resource was in an unexpected state


<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_EROLE_ID">EROLE_ID</a>: u64 = 0;
</code></pre>



<a name="0x1_Roles_EVALIDATOR"></a>

The signer didn't have the required Validator role


<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_EVALIDATOR">EVALIDATOR</a>: u64 = 7;
</code></pre>



<a name="0x1_Roles_EVALIDATOR_OPERATOR"></a>

The signer didn't have the required Validator Operator role


<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_EVALIDATOR_OPERATOR">EVALIDATOR_OPERATOR</a>: u64 = 8;
</code></pre>



<a name="0x1_Roles_NO_ROLE_ID"></a>



<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_NO_ROLE_ID">NO_ROLE_ID</a>: u64 = 100;
</code></pre>



<a name="0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID"></a>



<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID">VALIDATOR_OPERATOR_ROLE_ID</a>: u64 = 4;
</code></pre>



<a name="0x1_Roles_VALIDATOR_ROLE_ID"></a>



<pre><code><b>const</b> <a href="Roles.md#0x1_Roles_VALIDATOR_ROLE_ID">VALIDATOR_ROLE_ID</a>: u64 = 3;
</code></pre>



<a name="0x1_Roles_grant_diem_root_role"></a>

## Function `grant_diem_root_role`

Publishes diem root role. Granted only in genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_grant_diem_root_role">grant_diem_root_role</a>(dr_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_grant_diem_root_role">grant_diem_root_role</a>(
    dr_account: &signer,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_genesis">DiemTimestamp::assert_genesis</a>();
    // Checks actual Diem root because Diem root role is not set
    // until next line of code.
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    // Grant the role <b>to</b> the diem root account
    <a href="Roles.md#0x1_Roles_grant_role">grant_role</a>(dr_account, <a href="Roles.md#0x1_Roles_DIEM_ROOT_ROLE_ID">DIEM_ROOT_ROLE_ID</a>);
}
</code></pre>



</details>

<a name="0x1_Roles_new_validator_role"></a>

## Function `new_validator_role`

Publish a Validator <code><a href="Roles.md#0x1_Roles_RoleId">RoleId</a></code> under <code>new_account</code>.
The <code>creating_account</code> must be diem root.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_new_validator_role">new_validator_role</a>(creating_account: &signer, new_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_new_validator_role">new_validator_role</a>(
    creating_account: &signer,
    new_account: &signer
) <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_assert_diem_root">assert_diem_root</a>(creating_account);
    <a href="Roles.md#0x1_Roles_grant_role">grant_role</a>(new_account, <a href="Roles.md#0x1_Roles_VALIDATOR_ROLE_ID">VALIDATOR_ROLE_ID</a>);
}
</code></pre>



</details>

<a name="0x1_Roles_new_validator_operator_role"></a>

## Function `new_validator_operator_role`

Publish a ValidatorOperator <code><a href="Roles.md#0x1_Roles_RoleId">RoleId</a></code> under <code>new_account</code>.
The <code>creating_account</code> must be DiemRoot


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_new_validator_operator_role">new_validator_operator_role</a>(creating_account: &signer, new_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Roles.md#0x1_Roles_new_validator_operator_role">new_validator_operator_role</a>(
    creating_account: &signer,
    new_account: &signer,
) <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_assert_diem_root">assert_diem_root</a>(creating_account);
    <a href="Roles.md#0x1_Roles_grant_role">grant_role</a>(new_account, <a href="Roles.md#0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID">VALIDATOR_OPERATOR_ROLE_ID</a>);
}
</code></pre>



</details>

<a name="0x1_Roles_grant_role"></a>

## Function `grant_role`

Helper function to grant a role.


<pre><code><b>fun</b> <a href="Roles.md#0x1_Roles_grant_role">grant_role</a>(account: &signer, role_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Roles.md#0x1_Roles_grant_role">grant_role</a>(account: &signer, role_id: u64) {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account)), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="Roles.md#0x1_Roles_EROLE_ID">EROLE_ID</a>));
    <b>move_to</b>(account, <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> { role_id });
}
</code></pre>



</details>

<a name="0x1_Roles_has_role"></a>

## Function `has_role`



<pre><code><b>fun</b> <a href="Roles.md#0x1_Roles_has_role">has_role</a>(account: &signer, role_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Roles.md#0x1_Roles_has_role">has_role</a>(account: &signer, role_id: u64): bool <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_get_role_id">get_role_id</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account)) == role_id
}
</code></pre>



</details>

<a name="0x1_Roles_has_diem_root_role"></a>

## Function `has_diem_root_role`



<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_diem_root_role">has_diem_root_role</a>(account: &signer): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_diem_root_role">has_diem_root_role</a>(account: &signer): bool <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_has_role">has_role</a>(account, <a href="Roles.md#0x1_Roles_DIEM_ROOT_ROLE_ID">DIEM_ROOT_ROLE_ID</a>)
}
</code></pre>



</details>

<a name="0x1_Roles_has_validator_role"></a>

## Function `has_validator_role`



<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_validator_role">has_validator_role</a>(account: &signer): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_validator_role">has_validator_role</a>(account: &signer): bool <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_has_role">has_role</a>(account, <a href="Roles.md#0x1_Roles_VALIDATOR_ROLE_ID">VALIDATOR_ROLE_ID</a>)
}
</code></pre>



</details>

<a name="0x1_Roles_has_validator_operator_role"></a>

## Function `has_validator_operator_role`



<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_validator_operator_role">has_validator_operator_role</a>(account: &signer): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_has_validator_operator_role">has_validator_operator_role</a>(account: &signer): bool <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="Roles.md#0x1_Roles_has_role">has_role</a>(account, <a href="Roles.md#0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID">VALIDATOR_OPERATOR_ROLE_ID</a>)
}
</code></pre>



</details>

<a name="0x1_Roles_get_role_id"></a>

## Function `get_role_id`



<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_get_role_id">get_role_id</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_get_role_id">get_role_id</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(addr)) {
        <b>borrow_global</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(addr).role_id
    } <b>else</b> {
        <a href="Roles.md#0x1_Roles_NO_ROLE_ID">NO_ROLE_ID</a>
    }
}
</code></pre>



</details>

<a name="0x1_Roles_assert_diem_root"></a>

## Function `assert_diem_root`

Assert that the account is diem root.


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_diem_root">assert_diem_root</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_diem_root">assert_diem_root</a>(account: &signer) <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(account);
    <b>let</b> addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>assert</b>!(<b>exists</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(addr), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="Roles.md#0x1_Roles_EROLE_ID">EROLE_ID</a>));
    <b>assert</b>!(<b>borrow_global</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(addr).role_id == <a href="Roles.md#0x1_Roles_DIEM_ROOT_ROLE_ID">DIEM_ROOT_ROLE_ID</a>, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_requires_role">Errors::requires_role</a>(<a href="Roles.md#0x1_Roles_EDIEM_ROOT">EDIEM_ROOT</a>));
}
</code></pre>



</details>

<a name="0x1_Roles_assert_validator"></a>

## Function `assert_validator`

Assert that the account has the validator role.


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_validator">assert_validator</a>(validator_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_validator">assert_validator</a>(validator_account: &signer) <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <b>let</b> validator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(validator_account);
    <b>assert</b>!(<b>exists</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(validator_addr), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="Roles.md#0x1_Roles_EROLE_ID">EROLE_ID</a>));
    <b>assert</b>!(
        <b>borrow_global</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(validator_addr).role_id == <a href="Roles.md#0x1_Roles_VALIDATOR_ROLE_ID">VALIDATOR_ROLE_ID</a>,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_requires_role">Errors::requires_role</a>(<a href="Roles.md#0x1_Roles_EVALIDATOR">EVALIDATOR</a>)
    )
}
</code></pre>



</details>

<a name="0x1_Roles_assert_validator_operator"></a>

## Function `assert_validator_operator`

Assert that the account has the validator operator role.


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_validator_operator">assert_validator_operator</a>(validator_operator_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Roles.md#0x1_Roles_assert_validator_operator">assert_validator_operator</a>(validator_operator_account: &signer) <b>acquires</b> <a href="Roles.md#0x1_Roles_RoleId">RoleId</a> {
    <b>let</b> validator_operator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(validator_operator_account);
    <b>assert</b>!(<b>exists</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(validator_operator_addr), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="Roles.md#0x1_Roles_EROLE_ID">EROLE_ID</a>));
    <b>assert</b>!(
        <b>borrow_global</b>&lt;<a href="Roles.md#0x1_Roles_RoleId">RoleId</a>&gt;(validator_operator_addr).role_id == <a href="Roles.md#0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID">VALIDATOR_OPERATOR_ROLE_ID</a>,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_requires_role">Errors::requires_role</a>(<a href="Roles.md#0x1_Roles_EVALIDATOR_OPERATOR">EVALIDATOR_OPERATOR</a>)
    )
}
</code></pre>



</details>
