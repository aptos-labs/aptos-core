
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


<pre><code>use 0x1::bcs;
use 0x1::error;
use 0x1::from_bcs;
use 0x1::string;
use 0x1::type_info;
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


<pre><code>struct Any has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>data: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_any_ETYPE_MISMATCH"></a>

The type provided for <code>unpack</code> is not the same as was given for <code>pack</code>.


<pre><code>const ETYPE_MISMATCH: u64 &#61; 1;
</code></pre>



<a id="0x1_any_pack"></a>

## Function `pack`

Pack a value into the <code>Any</code> representation. Because Any can be stored and dropped, this is
also required from <code>T</code>.


<pre><code>public fun pack&lt;T: drop, store&gt;(x: T): any::Any
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pack&lt;T: drop &#43; store&gt;(x: T): Any &#123;
    Any &#123;
        type_name: type_info::type_name&lt;T&gt;(),
        data: to_bytes(&amp;x)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_any_unpack"></a>

## Function `unpack`

Unpack a value from the <code>Any</code> representation. This aborts if the value has not the expected type <code>T</code>.


<pre><code>public fun unpack&lt;T&gt;(x: any::Any): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unpack&lt;T&gt;(x: Any): T &#123;
    assert!(type_info::type_name&lt;T&gt;() &#61;&#61; x.type_name, error::invalid_argument(ETYPE_MISMATCH));
    from_bytes&lt;T&gt;(x.data)
&#125;
</code></pre>



</details>

<a id="0x1_any_type_name"></a>

## Function `type_name`

Returns the type name of this Any


<pre><code>public fun type_name(x: &amp;any::Any): &amp;string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun type_name(x: &amp;Any): &amp;String &#123;
    &amp;x.type_name
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_pack"></a>

### Function `pack`


<pre><code>public fun pack&lt;T: drop, store&gt;(x: T): any::Any
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; Any &#123;
    type_name: type_info::type_name&lt;T&gt;(),
    data: bcs::serialize&lt;T&gt;(x)
&#125;;
ensures [abstract] from_bcs::deserializable&lt;T&gt;(result.data);
</code></pre>



<a id="@Specification_1_unpack"></a>

### Function `unpack`


<pre><code>public fun unpack&lt;T&gt;(x: any::Any): T
</code></pre>




<pre><code>include UnpackAbortsIf&lt;T&gt;;
ensures result &#61;&#61; from_bcs::deserialize&lt;T&gt;(x.data);
</code></pre>




<a id="0x1_any_UnpackAbortsIf"></a>


<pre><code>schema UnpackAbortsIf&lt;T&gt; &#123;
    x: Any;
    aborts_if type_info::type_name&lt;T&gt;() !&#61; x.type_name;
    aborts_if !from_bcs::deserializable&lt;T&gt;(x.data);
&#125;
</code></pre>




<a id="0x1_any_UnpackRequirement"></a>


<pre><code>schema UnpackRequirement&lt;T&gt; &#123;
    x: Any;
    requires type_info::type_name&lt;T&gt;() &#61;&#61; x.type_name;
    requires from_bcs::deserializable&lt;T&gt;(x.data);
&#125;
</code></pre>



<a id="@Specification_1_type_name"></a>

### Function `type_name`


<pre><code>public fun type_name(x: &amp;any::Any): &amp;string::String
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; x.type_name;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
