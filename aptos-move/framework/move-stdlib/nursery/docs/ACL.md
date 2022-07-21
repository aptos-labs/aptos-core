
<a name="0x1_ACL"></a>

# Module `0x1::ACL`

Access control list (ACL) module. An ACL is a list of account addresses who
have the access permission to a certain object.
This module uses a <code>vector</code> to represent the list, but can be refactored to
use a "set" instead when it's available in the language in the future.


-  [Struct `ACL`](#0x1_ACL_ACL)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x1_ACL_empty)
-  [Function `add`](#0x1_ACL_add)
-  [Function `remove`](#0x1_ACL_remove)
-  [Function `contains`](#0x1_ACL_contains)
-  [Function `assert_contains`](#0x1_ACL_assert_contains)


<pre><code><b>use</b> <a href="">0x1::Errors</a>;
<b>use</b> <a href="">0x1::Vector</a>;
</code></pre>



<a name="0x1_ACL_ACL"></a>

## Struct `ACL`



<pre><code><b>struct</b> <a href="ACL.md#0x1_ACL">ACL</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>list: vector&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_ACL_ECONTAIN"></a>

The ACL already contains the address.


<pre><code><b>const</b> <a href="ACL.md#0x1_ACL_ECONTAIN">ECONTAIN</a>: u64 = 0;
</code></pre>



<a name="0x1_ACL_ENOT_CONTAIN"></a>

The ACL does not contain the address.


<pre><code><b>const</b> <a href="ACL.md#0x1_ACL_ENOT_CONTAIN">ENOT_CONTAIN</a>: u64 = 1;
</code></pre>



<a name="0x1_ACL_empty"></a>

## Function `empty`

Return an empty ACL.


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_empty">empty</a>(): <a href="ACL.md#0x1_ACL_ACL">ACL::ACL</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_empty">empty</a>(): <a href="ACL.md#0x1_ACL">ACL</a> {
    <a href="ACL.md#0x1_ACL">ACL</a>{ list: <a href="_empty">Vector::empty</a>&lt;<b>address</b>&gt;() }
}
</code></pre>



</details>

<a name="0x1_ACL_add"></a>

## Function `add`

Add the address to the ACL.


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_add">add</a>(acl: &<b>mut</b> <a href="ACL.md#0x1_ACL_ACL">ACL::ACL</a>, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_add">add</a>(acl: &<b>mut</b> <a href="ACL.md#0x1_ACL">ACL</a>, addr: <b>address</b>) {
    <b>assert</b>!(!<a href="_contains">Vector::contains</a>(&<b>mut</b> acl.list, &addr), <a href="_invalid_argument">Errors::invalid_argument</a>(<a href="ACL.md#0x1_ACL_ECONTAIN">ECONTAIN</a>));
    <a href="_push_back">Vector::push_back</a>(&<b>mut</b> acl.list, addr);
}
</code></pre>



</details>

<a name="0x1_ACL_remove"></a>

## Function `remove`

Remove the address from the ACL.


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_remove">remove</a>(acl: &<b>mut</b> <a href="ACL.md#0x1_ACL_ACL">ACL::ACL</a>, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_remove">remove</a>(acl: &<b>mut</b> <a href="ACL.md#0x1_ACL">ACL</a>, addr: <b>address</b>) {
    <b>let</b> (found, index) = <a href="_index_of">Vector::index_of</a>(&<b>mut</b> acl.list, &addr);
    <b>assert</b>!(found, <a href="_invalid_argument">Errors::invalid_argument</a>(<a href="ACL.md#0x1_ACL_ENOT_CONTAIN">ENOT_CONTAIN</a>));
    <a href="_remove">Vector::remove</a>(&<b>mut</b> acl.list, index);
}
</code></pre>



</details>

<a name="0x1_ACL_contains"></a>

## Function `contains`

Return true iff the ACL contains the address.


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_contains">contains</a>(acl: &<a href="ACL.md#0x1_ACL_ACL">ACL::ACL</a>, addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_contains">contains</a>(acl: &<a href="ACL.md#0x1_ACL">ACL</a>, addr: <b>address</b>): bool {
    <a href="_contains">Vector::contains</a>(&acl.list, &addr)
}
</code></pre>



</details>

<a name="0x1_ACL_assert_contains"></a>

## Function `assert_contains`

assert! that the ACL has the address.


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_assert_contains">assert_contains</a>(acl: &<a href="ACL.md#0x1_ACL_ACL">ACL::ACL</a>, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ACL.md#0x1_ACL_assert_contains">assert_contains</a>(acl: &<a href="ACL.md#0x1_ACL">ACL</a>, addr: <b>address</b>) {
    <b>assert</b>!(<a href="ACL.md#0x1_ACL_contains">contains</a>(acl, addr), <a href="_invalid_argument">Errors::invalid_argument</a>(<a href="ACL.md#0x1_ACL_ENOT_CONTAIN">ENOT_CONTAIN</a>));
}
</code></pre>



</details>
