
<a id="0x4_royalty"></a>

# Module `0x4::royalty`

This defines an object-based Royalty. The royalty can be applied to either a collection or a
token. Applications should read the royalty from the token, as it will read the appropriate
royalty.


-  [Resource `Royalty`](#0x4_royalty_Royalty)
-  [Struct `MutatorRef`](#0x4_royalty_MutatorRef)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x4_royalty_init)
-  [Function `update`](#0x4_royalty_update)
-  [Function `create`](#0x4_royalty_create)
-  [Function `generate_mutator_ref`](#0x4_royalty_generate_mutator_ref)
-  [Function `exists_at`](#0x4_royalty_exists_at)
-  [Function `delete`](#0x4_royalty_delete)
-  [Function `get`](#0x4_royalty_get)
-  [Function `denominator`](#0x4_royalty_denominator)
-  [Function `numerator`](#0x4_royalty_numerator)
-  [Function `payee_address`](#0x4_royalty_payee_address)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x4_royalty_Royalty"></a>

## Resource `Royalty`

The royalty of a token within this collection

Royalties are optional for a collection.  Royalty percentage is calculated
by (numerator / denominator) * 100%


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>payee_address: <b>address</b></code>
</dt>
<dd>
 The recipient of royalty payments. See the <code>shared_account</code> for how to handle multiple
 creators.
</dd>
</dl>


</details>

<a id="0x4_royalty_MutatorRef"></a>

## Struct `MutatorRef`

This enables creating or overwriting a <code><a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a></code>.


<pre><code><b>struct</b> <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO"></a>

The royalty denominator cannot be 0


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO">EROYALTY_DENOMINATOR_IS_ZERO</a>: u64 = 3;
</code></pre>



<a id="0x4_royalty_EROYALTY_DOES_NOT_EXIST"></a>

Royalty does not exist


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_DOES_NOT_EXIST">EROYALTY_DOES_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a id="0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM"></a>

The royalty cannot be greater than 100%


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM">EROYALTY_EXCEEDS_MAXIMUM</a>: u64 = 2;
</code></pre>



<a id="0x4_royalty_init"></a>

## Function `init`

Add a royalty, given a ConstructorRef.


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_init">init</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_init">init</a>(ref: &ConstructorRef, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">Royalty</a>) {
    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(ref);
    <b>move_to</b>(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="royalty.md#0x4_royalty">royalty</a>);
}
</code></pre>



</details>

<a id="0x4_royalty_update"></a>

## Function `update`

Set the royalty if it does not exist, replace it otherwise.


<pre><code><b>public</b> <b>fun</b> <b>update</b>(mutator_ref: &<a href="royalty.md#0x4_royalty_MutatorRef">royalty::MutatorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(mutator_ref: &<a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">Royalty</a>) <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> {
    <b>let</b> addr = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(&mutator_ref.inner);
    <b>if</b> (<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr)) {
        <b>move_from</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr);
    };

    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&mutator_ref.inner);
    <b>move_to</b>(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="royalty.md#0x4_royalty">royalty</a>);
}
</code></pre>



</details>

<a id="0x4_royalty_create"></a>

## Function `create`

Creates a new royalty, verifying that it is a valid percentage


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_create">create</a>(numerator: u64, denominator: u64, payee_address: <b>address</b>): <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_create">create</a>(numerator: u64, denominator: u64, payee_address: <b>address</b>): <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> {
    <b>assert</b>!(denominator != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="royalty.md#0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO">EROYALTY_DENOMINATOR_IS_ZERO</a>));
    <b>assert</b>!(<a href="royalty.md#0x4_royalty_numerator">numerator</a> &lt;= denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="royalty.md#0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM">EROYALTY_EXCEEDS_MAXIMUM</a>));

    <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> { numerator, denominator, payee_address }
}
</code></pre>



</details>

<a id="0x4_royalty_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_generate_mutator_ref">generate_mutator_ref</a>(ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a>): <a href="royalty.md#0x4_royalty_MutatorRef">royalty::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_generate_mutator_ref">generate_mutator_ref</a>(ref: ExtendRef): <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> {
    <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> { inner: ref }
}
</code></pre>



</details>

<a id="0x4_royalty_exists_at"></a>

## Function `exists_at`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x4_royalty_delete"></a>

## Function `delete`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="royalty.md#0x4_royalty_delete">delete</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="royalty.md#0x4_royalty_delete">delete</a>(addr: <b>address</b>) <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="royalty.md#0x4_royalty_EROYALTY_DOES_NOT_EXIST">EROYALTY_DOES_NOT_EXIST</a>));
    <b>move_from</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr);
}
</code></pre>



</details>

<a id="0x4_royalty_get"></a>

## Function `get`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_get">get</a>&lt;T: key&gt;(maybe_royalty: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_get">get</a>&lt;T: key&gt;(maybe_royalty: Object&lt;T&gt;): Option&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt; <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> {
    <b>let</b> obj_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&maybe_royalty);
    <b>if</b> (<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(obj_addr)) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>[obj_addr])
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x4_royalty_denominator"></a>

## Function `denominator`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_denominator">denominator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_denominator">denominator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): u64 {
    <a href="royalty.md#0x4_royalty">royalty</a>.denominator
}
</code></pre>



</details>

<a id="0x4_royalty_numerator"></a>

## Function `numerator`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_numerator">numerator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_numerator">numerator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): u64 {
    <a href="royalty.md#0x4_royalty">royalty</a>.numerator
}
</code></pre>



</details>

<a id="0x4_royalty_payee_address"></a>

## Function `payee_address`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_payee_address">payee_address</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_payee_address">payee_address</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): <b>address</b> {
    <a href="royalty.md#0x4_royalty">royalty</a>.payee_address
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
