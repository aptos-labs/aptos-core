
<a id="0x1_type_info"></a>

# Module `0x1::type_info`



-  [Struct `TypeInfo`](#0x1_type_info_TypeInfo)
-  [Constants](#@Constants_0)
-  [Function `account_address`](#0x1_type_info_account_address)
-  [Function `module_name`](#0x1_type_info_module_name)
-  [Function `struct_name`](#0x1_type_info_struct_name)
-  [Function `chain_id`](#0x1_type_info_chain_id)
-  [Function `type_of`](#0x1_type_info_type_of)
-  [Function `type_name`](#0x1_type_info_type_name)
-  [Function `chain_id_internal`](#0x1_type_info_chain_id_internal)
-  [Function `size_of_val`](#0x1_type_info_size_of_val)
-  [Function `verify_type_of`](#0x1_type_info_verify_type_of)
-  [Function `verify_type_of_generic`](#0x1_type_info_verify_type_of_generic)
-  [Specification](#@Specification_1)
    -  [Function `chain_id`](#@Specification_1_chain_id)
    -  [Function `type_of`](#@Specification_1_type_of)
    -  [Function `type_name`](#@Specification_1_type_name)
    -  [Function `chain_id_internal`](#@Specification_1_chain_id_internal)
    -  [Function `size_of_val`](#@Specification_1_size_of_val)
    -  [Function `verify_type_of_generic`](#@Specification_1_verify_type_of_generic)


<pre><code>use 0x1::bcs;<br/>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_type_info_TypeInfo"></a>

## Struct `TypeInfo`



<pre><code>struct TypeInfo has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>struct_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_type_info_E_NATIVE_FUN_NOT_AVAILABLE"></a>



<pre><code>const E_NATIVE_FUN_NOT_AVAILABLE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_type_info_account_address"></a>

## Function `account_address`



<pre><code>public fun account_address(type_info: &amp;type_info::TypeInfo): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun account_address(type_info: &amp;TypeInfo): address &#123;<br/>    type_info.account_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_module_name"></a>

## Function `module_name`



<pre><code>public fun module_name(type_info: &amp;type_info::TypeInfo): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun module_name(type_info: &amp;TypeInfo): vector&lt;u8&gt; &#123;<br/>    type_info.module_name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_struct_name"></a>

## Function `struct_name`



<pre><code>public fun struct_name(type_info: &amp;type_info::TypeInfo): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun struct_name(type_info: &amp;TypeInfo): vector&lt;u8&gt; &#123;<br/>    type_info.struct_name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_chain_id"></a>

## Function `chain_id`

Returns the current chain ID, mirroring what <code>aptos_framework::chain_id::get()</code> would return, except in <code>&#35;[test]</code><br/> functions, where this will always return <code>4u8</code> as the chain ID, whereas <code>aptos_framework::chain_id::get()</code> will<br/> return whichever ID was passed to <code>aptos_framework::chain_id::initialize_for_test()</code>.


<pre><code>public fun chain_id(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun chain_id(): u8 &#123;<br/>    if (!features::aptos_stdlib_chain_id_enabled()) &#123;<br/>        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))<br/>    &#125;;<br/><br/>    chain_id_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_type_of"></a>

## Function `type_of`

Return the <code>TypeInfo</code> struct containing  for the type <code>T</code>.


<pre><code>public fun type_of&lt;T&gt;(): type_info::TypeInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun type_of&lt;T&gt;(): TypeInfo;<br/></code></pre>



</details>

<a id="0x1_type_info_type_name"></a>

## Function `type_name`

Return the human readable string for the type, including the address, module name, and any type arguments.<br/> Example: 0x1::coin::CoinStore&lt;0x1::aptos_coin::AptosCoin&gt;<br/> Or: 0x1::table::Table&lt;0x1::string::String, 0x1::string::String&gt;


<pre><code>public fun type_name&lt;T&gt;(): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun type_name&lt;T&gt;(): String;<br/></code></pre>



</details>

<a id="0x1_type_info_chain_id_internal"></a>

## Function `chain_id_internal`



<pre><code>fun chain_id_internal(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun chain_id_internal(): u8;<br/></code></pre>



</details>

<a id="0x1_type_info_size_of_val"></a>

## Function `size_of_val`

Return the BCS size, in bytes, of value at <code>val_ref</code>.<br/><br/> See the [BCS spec](https://github.com/diem/bcs)<br/><br/> See <code>test_size_of_val()</code> for an analysis of common types and<br/> nesting patterns, as well as <code>test_size_of_val_vectors()</code> for an<br/> analysis of vector size dynamism.


<pre><code>public fun size_of_val&lt;T&gt;(val_ref: &amp;T): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun size_of_val&lt;T&gt;(val_ref: &amp;T): u64 &#123;<br/>    // Return vector length of vectorized BCS representation.<br/>    vector::length(&amp;bcs::to_bytes(val_ref))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_verify_type_of"></a>

## Function `verify_type_of`



<pre><code>&#35;[verify_only]<br/>fun verify_type_of()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_type_of() &#123;<br/>    let type_info &#61; type_of&lt;TypeInfo&gt;();<br/>    let account_address &#61; account_address(&amp;type_info);<br/>    let module_name &#61; module_name(&amp;type_info);<br/>    let struct_name &#61; struct_name(&amp;type_info);<br/>    spec &#123;<br/>        assert account_address &#61;&#61; @aptos_std;<br/>        assert module_name &#61;&#61; b&quot;type_info&quot;;<br/>        assert struct_name &#61;&#61; b&quot;TypeInfo&quot;;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_type_info_verify_type_of_generic"></a>

## Function `verify_type_of_generic`



<pre><code>&#35;[verify_only]<br/>fun verify_type_of_generic&lt;T&gt;()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_type_of_generic&lt;T&gt;() &#123;<br/>    let type_info &#61; type_of&lt;T&gt;();<br/>    let account_address &#61; account_address(&amp;type_info);<br/>    let module_name &#61; module_name(&amp;type_info);<br/>    let struct_name &#61; struct_name(&amp;type_info);<br/>    spec &#123;<br/>        assert account_address &#61;&#61; type_of&lt;T&gt;().account_address;<br/>        assert module_name &#61;&#61; type_of&lt;T&gt;().module_name;<br/>        assert struct_name &#61;&#61; type_of&lt;T&gt;().struct_name;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_chain_id"></a>

### Function `chain_id`


<pre><code>public fun chain_id(): u8<br/></code></pre>




<pre><code>aborts_if !features::spec_is_enabled(features::APTOS_STD_CHAIN_ID_NATIVES);<br/>ensures result &#61;&#61; spec_chain_id_internal();<br/></code></pre>



<a id="@Specification_1_type_of"></a>

### Function `type_of`


<pre><code>public fun type_of&lt;T&gt;(): type_info::TypeInfo<br/></code></pre>




<a id="@Specification_1_type_name"></a>

### Function `type_name`


<pre><code>public fun type_name&lt;T&gt;(): string::String<br/></code></pre>




<a id="@Specification_1_chain_id_internal"></a>

### Function `chain_id_internal`


<pre><code>fun chain_id_internal(): u8<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_chain_id_internal();<br/></code></pre>




<a id="0x1_type_info_spec_chain_id_internal"></a>


<pre><code>fun spec_chain_id_internal(): u8;<br/></code></pre>




<a id="0x1_type_info_spec_size_of_val"></a>


<pre><code>fun spec_size_of_val&lt;T&gt;(val_ref: T): u64 &#123;<br/>   len(std::bcs::serialize(val_ref))<br/>&#125;<br/></code></pre>



<a id="@Specification_1_size_of_val"></a>

### Function `size_of_val`


<pre><code>public fun size_of_val&lt;T&gt;(val_ref: &amp;T): u64<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_size_of_val&lt;T&gt;(val_ref);<br/></code></pre>



<a id="@Specification_1_verify_type_of_generic"></a>

### Function `verify_type_of_generic`


<pre><code>&#35;[verify_only]<br/>fun verify_type_of_generic&lt;T&gt;()<br/></code></pre>




<pre><code>aborts_if !spec_is_struct&lt;T&gt;();<br/></code></pre>




<a id="0x1_type_info_spec_is_struct"></a>


<pre><code>native fun spec_is_struct&lt;T&gt;(): bool;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
