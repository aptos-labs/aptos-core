
<a name="0x1_Role"></a>

# Module `0x1::Role`

A generic module for role-based access control (RBAC).


-  [Resource `Role`](#0x1_Role_Role)
-  [Constants](#@Constants_0)
-  [Function `assign_role`](#0x1_Role_assign_role)
-  [Function `revoke_role`](#0x1_Role_revoke_role)
-  [Function `has_role`](#0x1_Role_has_role)
-  [Function `assert_has_role`](#0x1_Role_assert_has_role)


<pre><code><b>use</b> <a href="">0x1::Errors</a>;
<b>use</b> <a href="">0x1::Signer</a>;
</code></pre>



<a name="0x1_Role_Role"></a>

## Resource `Role`



<pre><code><b>struct</b> <a href="Role.md#0x1_Role">Role</a>&lt;Type&gt; has key
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Role_EROLE"></a>



<pre><code><b>const</b> <a href="Role.md#0x1_Role_EROLE">EROLE</a>: u64 = 0;
</code></pre>



<a name="0x1_Role_assign_role"></a>

## Function `assign_role`

Assign the role to the account. The caller must pass a witness, so is
expected to be a function of the module that defines <code>Type</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_assign_role">assign_role</a>&lt;Type&gt;(<b>to</b>: &signer, _witness: &Type)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_assign_role">assign_role</a>&lt;Type&gt;(<b>to</b>: &signer, _witness: &Type) {
    <b>assert</b>!(!<a href="Role.md#0x1_Role_has_role">has_role</a>&lt;Type&gt;(<a href="_address_of">Signer::address_of</a>(<b>to</b>)), <a href="_already_published">Errors::already_published</a>(<a href="Role.md#0x1_Role_EROLE">EROLE</a>));
    move_to&lt;<a href="Role.md#0x1_Role">Role</a>&lt;Type&gt;&gt;(<b>to</b>, <a href="Role.md#0x1_Role">Role</a>&lt;Type&gt;{});
}
</code></pre>



</details>

<a name="0x1_Role_revoke_role"></a>

## Function `revoke_role`

Revoke the role from the account. The caller must pass a witness, so is
expected to be a function of the module that defines <code>Type</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_revoke_role">revoke_role</a>&lt;Type&gt;(from: &signer, _witness: &Type)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_revoke_role">revoke_role</a>&lt;Type&gt;(from: &signer, _witness: &Type) <b>acquires</b> <a href="Role.md#0x1_Role">Role</a> {
    <b>assert</b>!(<a href="Role.md#0x1_Role_has_role">has_role</a>&lt;Type&gt;(<a href="_address_of">Signer::address_of</a>(from)), <a href="_not_published">Errors::not_published</a>(<a href="Role.md#0x1_Role_EROLE">EROLE</a>));
    <b>let</b> <a href="Role.md#0x1_Role">Role</a>&lt;Type&gt;{} = move_from&lt;<a href="Role.md#0x1_Role">Role</a>&lt;Type&gt;&gt;(<a href="_address_of">Signer::address_of</a>(from));
}
</code></pre>



</details>

<a name="0x1_Role_has_role"></a>

## Function `has_role`

Return true iff the address has the role.


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_has_role">has_role</a>&lt;Type&gt;(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_has_role">has_role</a>&lt;Type&gt;(addr: address): bool {
    <b>exists</b>&lt;<a href="Role.md#0x1_Role">Role</a>&lt;Type&gt;&gt;(addr)
}
</code></pre>



</details>

<a name="0x1_Role_assert_has_role"></a>

## Function `assert_has_role`

assert! that the account has the role.


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_assert_has_role">assert_has_role</a>&lt;Type&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Role.md#0x1_Role_assert_has_role">assert_has_role</a>&lt;Type&gt;(account: &signer) {
    <b>assert</b>!(<a href="Role.md#0x1_Role_has_role">has_role</a>&lt;Type&gt;(<a href="_address_of">Signer::address_of</a>(account)), <a href="_not_published">Errors::not_published</a>(<a href="Role.md#0x1_Role_EROLE">EROLE</a>));
}
</code></pre>



</details>
