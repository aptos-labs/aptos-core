
<a id="0x1_any"></a>

# Module `0x1::any`



-  [Struct `Any`](#0x1_any_Any)
-  [Constants](#@Constants_0)
-  [Function `pack`](#0x1_any_pack)
-  [Function `unpack`](#0x1_any_unpack)
-  [Function `type_name`](#0x1_any_type_name)
-  [Specification](#@Specification_1)
    -  [Function `pack`](#@Specification_1_pack)
    -  [Function `unpack`](#@Specification_1_unpack)
    -  [Function `type_name`](#@Specification_1_type_name)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a id="0x1_any_Any"></a>

## Struct `Any`

A type which can represent a value of any type. This allows for representation of 'unknown' future
values. For example, to define a resource such that it can be later be extended without breaking
changes one can do

```move
struct Resource {
field: Type,
...
extension: Option<Any>
}
```


<pre><code><b>struct</b> <a href="any.md#0x1_any_Any">Any</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type_name: <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_any_ETYPE_MISMATCH"></a>

The type provided for <code>unpack</code> is not the same as was given for <code>pack</code>.


<pre><code><b>const</b> <a href="any.md#0x1_any_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_any_pack"></a>

## Function `pack`

Pack a value into the <code><a href="any.md#0x1_any_Any">Any</a></code> representation. Because Any can be stored and dropped, this is
also required from <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_pack">pack</a>&lt;T: drop, store&gt;(x: T): <a href="any.md#0x1_any_Any">any::Any</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_pack">pack</a>&lt;T: drop + store&gt;(x: T): <a href="any.md#0x1_any_Any">Any</a> {
    <a href="any.md#0x1_any_Any">Any</a> {
        type_name: <a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;(),
        data: to_bytes(&x)
    }
}
</code></pre>



</details>

<a id="0x1_any_unpack"></a>

## Function `unpack`

Unpack a value from the <code><a href="any.md#0x1_any_Any">Any</a></code> representation. This aborts if the value has not the expected type <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_unpack">unpack</a>&lt;T&gt;(self: <a href="any.md#0x1_any_Any">any::Any</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_unpack">unpack</a>&lt;T&gt;(self: <a href="any.md#0x1_any_Any">Any</a>): T {
    <b>assert</b>!(<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;() == self.type_name, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="any.md#0x1_any_ETYPE_MISMATCH">ETYPE_MISMATCH</a>));
    from_bytes&lt;T&gt;(self.data)
}
</code></pre>



</details>

<a id="0x1_any_type_name"></a>

## Function `type_name`

Returns the type name of this Any


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_type_name">type_name</a>(self: &<a href="any.md#0x1_any_Any">any::Any</a>): &<a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_type_name">type_name</a>(self: &<a href="any.md#0x1_any_Any">Any</a>): &String {
    &self.type_name
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_pack"></a>

### Function `pack`


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_pack">pack</a>&lt;T: drop, store&gt;(x: T): <a href="any.md#0x1_any_Any">any::Any</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="any.md#0x1_any_Any">Any</a> {
    type_name: <a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;(),
    data: <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_serialize">bcs::serialize</a>&lt;T&gt;(x)
};
<b>ensures</b> [abstract] <a href="from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;T&gt;(result.data);
</code></pre>



<a id="@Specification_1_unpack"></a>

### Function `unpack`


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_unpack">unpack</a>&lt;T&gt;(self: <a href="any.md#0x1_any_Any">any::Any</a>): T
</code></pre>




<pre><code><b>include</b> <a href="any.md#0x1_any_UnpackAbortsIf">UnpackAbortsIf</a>&lt;T&gt;;
<b>ensures</b> result == <a href="from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;T&gt;(self.data);
</code></pre>




<a id="0x1_any_UnpackAbortsIf"></a>


<pre><code><b>schema</b> <a href="any.md#0x1_any_UnpackAbortsIf">UnpackAbortsIf</a>&lt;T&gt; {
    self: <a href="any.md#0x1_any_Any">Any</a>;
    <b>aborts_if</b> <a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;() != self.type_name;
    <b>aborts_if</b> !<a href="from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;T&gt;(self.data);
}
</code></pre>




<a id="0x1_any_UnpackRequirement"></a>


<pre><code><b>schema</b> <a href="any.md#0x1_any_UnpackRequirement">UnpackRequirement</a>&lt;T&gt; {
    self: <a href="any.md#0x1_any_Any">Any</a>;
    <b>requires</b> <a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;() == self.type_name;
    <b>requires</b> <a href="from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;T&gt;(self.data);
}
</code></pre>



<a id="@Specification_1_type_name"></a>

### Function `type_name`


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any_type_name">type_name</a>(self: &<a href="any.md#0x1_any_Any">any::Any</a>): &<a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == self.type_name;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
