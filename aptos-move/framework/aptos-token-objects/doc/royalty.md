
<a id="0x4_royalty"></a>

# Module `0x4::royalty`

This defines an object&#45;based Royalty. The royalty can be applied to either a collection or a<br/> token. Applications should read the royalty from the token, as it will read the appropriate<br/> royalty.


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


<pre><code>use 0x1::error;<br/>use 0x1::object;<br/>use 0x1::option;<br/></code></pre>



<a id="0x4_royalty_Royalty"></a>

## Resource `Royalty`

The royalty of a token within this collection<br/><br/> Royalties are optional for a collection.  Royalty percentage is calculated<br/> by (numerator / denominator) &#42; 100%


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Royalty has copy, drop, key<br/></code></pre>



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
<code>payee_address: address</code>
</dt>
<dd>
 The recipient of royalty payments. See the <code>shared_account</code> for how to handle multiple<br/> creators.
</dd>
</dl>


</details>

<a id="0x4_royalty_MutatorRef"></a>

## Struct `MutatorRef`

This enables creating or overwriting a <code>MutatorRef</code>.


<pre><code>struct MutatorRef has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: object::ExtendRef</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x4_royalty_EROYALTY_DENOMINATOR_IS_ZERO"></a>

The royalty denominator cannot be 0


<pre><code>const EROYALTY_DENOMINATOR_IS_ZERO: u64 &#61; 3;<br/></code></pre>



<a id="0x4_royalty_EROYALTY_DOES_NOT_EXIST"></a>

Royalty does not exist


<pre><code>const EROYALTY_DOES_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x4_royalty_EROYALTY_EXCEEDS_MAXIMUM"></a>

The royalty cannot be greater than 100%


<pre><code>const EROYALTY_EXCEEDS_MAXIMUM: u64 &#61; 2;<br/></code></pre>



<a id="0x4_royalty_init"></a>

## Function `init`

Add a royalty, given a ConstructorRef.


<pre><code>public fun init(ref: &amp;object::ConstructorRef, royalty: royalty::Royalty)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun init(ref: &amp;ConstructorRef, royalty: Royalty) &#123;<br/>    let signer &#61; object::generate_signer(ref);<br/>    move_to(&amp;signer, royalty);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_update"></a>

## Function `update`

Set the royalty if it does not exist, replace it otherwise.


<pre><code>public fun update(mutator_ref: &amp;royalty::MutatorRef, royalty: royalty::Royalty)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update(mutator_ref: &amp;MutatorRef, royalty: Royalty) acquires Royalty &#123;<br/>    let addr &#61; object::address_from_extend_ref(&amp;mutator_ref.inner);<br/>    if (exists&lt;Royalty&gt;(addr)) &#123;<br/>        move_from&lt;Royalty&gt;(addr);<br/>    &#125;;<br/><br/>    let signer &#61; object::generate_signer_for_extending(&amp;mutator_ref.inner);<br/>    move_to(&amp;signer, royalty);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_create"></a>

## Function `create`

Creates a new royalty, verifying that it is a valid percentage


<pre><code>public fun create(numerator: u64, denominator: u64, payee_address: address): royalty::Royalty<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create(numerator: u64, denominator: u64, payee_address: address): Royalty &#123;<br/>    assert!(denominator !&#61; 0, error::out_of_range(EROYALTY_DENOMINATOR_IS_ZERO));<br/>    assert!(numerator &lt;&#61; denominator, error::out_of_range(EROYALTY_EXCEEDS_MAXIMUM));<br/><br/>    Royalty &#123; numerator, denominator, payee_address &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code>public fun generate_mutator_ref(ref: object::ExtendRef): royalty::MutatorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mutator_ref(ref: ExtendRef): MutatorRef &#123;<br/>    MutatorRef &#123; inner: ref &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_exists_at"></a>

## Function `exists_at`



<pre><code>public fun exists_at(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exists_at(addr: address): bool &#123;<br/>    exists&lt;Royalty&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_delete"></a>

## Function `delete`



<pre><code>public(friend) fun delete(addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun delete(addr: address) acquires Royalty &#123;<br/>    assert!(exists&lt;Royalty&gt;(addr), error::not_found(EROYALTY_DOES_NOT_EXIST));<br/>    move_from&lt;Royalty&gt;(addr);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_get"></a>

## Function `get`



<pre><code>public fun get&lt;T: key&gt;(maybe_royalty: object::Object&lt;T&gt;): option::Option&lt;royalty::Royalty&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get&lt;T: key&gt;(maybe_royalty: Object&lt;T&gt;): Option&lt;Royalty&gt; acquires Royalty &#123;<br/>    let obj_addr &#61; object::object_address(&amp;maybe_royalty);<br/>    if (exists&lt;Royalty&gt;(obj_addr)) &#123;<br/>        option::some(&#42;borrow_global&lt;Royalty&gt;(obj_addr))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_denominator"></a>

## Function `denominator`



<pre><code>public fun denominator(royalty: &amp;royalty::Royalty): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun denominator(royalty: &amp;Royalty): u64 &#123;<br/>    royalty.denominator<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_numerator"></a>

## Function `numerator`



<pre><code>public fun numerator(royalty: &amp;royalty::Royalty): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun numerator(royalty: &amp;Royalty): u64 &#123;<br/>    royalty.numerator<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_royalty_payee_address"></a>

## Function `payee_address`



<pre><code>public fun payee_address(royalty: &amp;royalty::Royalty): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun payee_address(royalty: &amp;Royalty): address &#123;<br/>    royalty.payee_address<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
