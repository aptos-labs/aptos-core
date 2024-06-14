
<a id="0x4_royalty"></a>

# Module `0x4::royalty`

This defines an object&#45;based Royalty. The royalty can be applied to either a collection or a
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


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /></code></pre>



<a id="0x4_royalty_Royalty"></a>

## Resource `Royalty`

The royalty of a token within this collection

Royalties are optional for a collection.  Royalty percentage is calculated
by (numerator / denominator) &#42; 100%


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> <b>has</b> <b>copy</b>, drop, key<br /></code></pre>



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


<pre><code><b>struct</b> <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO">EROYALTY_DENOMINATOR_IS_ZERO</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x4_royalty_EROYALTY_DOES_NOT_EXIST"></a>

Royalty does not exist


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_DOES_NOT_EXIST">EROYALTY_DOES_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM"></a>

The royalty cannot be greater than 100%


<pre><code><b>const</b> <a href="royalty.md#0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM">EROYALTY_EXCEEDS_MAXIMUM</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x4_royalty_init"></a>

## Function `init`

Add a royalty, given a ConstructorRef.


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_init">init</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_init">init</a>(ref: &amp;ConstructorRef, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">Royalty</a>) &#123;<br />    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(ref);<br />    <b>move_to</b>(&amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="royalty.md#0x4_royalty">royalty</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_update"></a>

## Function `update`

Set the royalty if it does not exist, replace it otherwise.


<pre><code><b>public</b> <b>fun</b> <b>update</b>(mutator_ref: &amp;<a href="royalty.md#0x4_royalty_MutatorRef">royalty::MutatorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(mutator_ref: &amp;<a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">Royalty</a>) <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(&amp;mutator_ref.inner);<br />    <b>if</b> (<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr)) &#123;<br />        <b>move_from</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr);<br />    &#125;;<br /><br />    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&amp;mutator_ref.inner);<br />    <b>move_to</b>(&amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="royalty.md#0x4_royalty">royalty</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_create"></a>

## Function `create`

Creates a new royalty, verifying that it is a valid percentage


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_create">create</a>(numerator: u64, denominator: u64, payee_address: <b>address</b>): <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_create">create</a>(numerator: u64, denominator: u64, payee_address: <b>address</b>): <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> &#123;<br />    <b>assert</b>!(denominator !&#61; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="royalty.md#0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO">EROYALTY_DENOMINATOR_IS_ZERO</a>));<br />    <b>assert</b>!(<a href="royalty.md#0x4_royalty_numerator">numerator</a> &lt;&#61; denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="royalty.md#0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM">EROYALTY_EXCEEDS_MAXIMUM</a>));<br /><br />    <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> &#123; numerator, denominator, payee_address &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_generate_mutator_ref">generate_mutator_ref</a>(ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a>): <a href="royalty.md#0x4_royalty_MutatorRef">royalty::MutatorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_generate_mutator_ref">generate_mutator_ref</a>(ref: ExtendRef): <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> &#123;<br />    <a href="royalty.md#0x4_royalty_MutatorRef">MutatorRef</a> &#123; inner: ref &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_exists_at"></a>

## Function `exists_at`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_exists_at">exists_at</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_exists_at">exists_at</a>(addr: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_delete"></a>

## Function `delete`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="royalty.md#0x4_royalty_delete">delete</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="royalty.md#0x4_royalty_delete">delete</a>(addr: <b>address</b>) <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="royalty.md#0x4_royalty_EROYALTY_DOES_NOT_EXIST">EROYALTY_DOES_NOT_EXIST</a>));<br />    <b>move_from</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(addr);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_get"></a>

## Function `get`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_get">get</a>&lt;T: key&gt;(maybe_royalty: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_get">get</a>&lt;T: key&gt;(maybe_royalty: Object&lt;T&gt;): Option&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt; <b>acquires</b> <a href="royalty.md#0x4_royalty_Royalty">Royalty</a> &#123;<br />    <b>let</b> obj_addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;maybe_royalty);<br />    <b>if</b> (<b>exists</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(obj_addr)) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;<b>borrow_global</b>&lt;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>&gt;(obj_addr))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_denominator"></a>

## Function `denominator`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_denominator">denominator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_denominator">denominator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): u64 &#123;<br />    <a href="royalty.md#0x4_royalty">royalty</a>.denominator<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_numerator"></a>

## Function `numerator`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_numerator">numerator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_numerator">numerator</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): u64 &#123;<br />    <a href="royalty.md#0x4_royalty">royalty</a>.numerator<br />&#125;<br /></code></pre>



</details>

<a id="0x4_royalty_payee_address"></a>

## Function `payee_address`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_payee_address">payee_address</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty_payee_address">payee_address</a>(<a href="royalty.md#0x4_royalty">royalty</a>: &amp;<a href="royalty.md#0x4_royalty_Royalty">Royalty</a>): <b>address</b> &#123;<br />    <a href="royalty.md#0x4_royalty">royalty</a>.payee_address<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
