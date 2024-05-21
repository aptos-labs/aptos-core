
<a id="0x1_from_bcs"></a>

# Module `0x1::from_bcs`

This module provides a number of functions to convert _primitive_ types from their representation in <code>std::bcs</code>
to values. This is the opposite of <code>bcs::to_bytes</code>. Note that it is not safe to define a generic public <code>from_bytes</code>
function because this can violate implicit struct invariants, therefore only primitive types are offerred. If
a general conversion back-and-force is needed, consider the <code>aptos_std::Any</code> type which preserves invariants.

Example:
```
use std::bcs;
use aptos_std::from_bcs;

assert!(from_bcs::to_address(bcs::to_bytes(&@0xabcdef)) == @0xabcdef, 0);
```


-  [Constants](#@Constants_0)
-  [Function `to_bool`](#0x1_from_bcs_to_bool)
-  [Function `to_u8`](#0x1_from_bcs_to_u8)
-  [Function `to_u16`](#0x1_from_bcs_to_u16)
-  [Function `to_u32`](#0x1_from_bcs_to_u32)
-  [Function `to_u64`](#0x1_from_bcs_to_u64)
-  [Function `to_u128`](#0x1_from_bcs_to_u128)
-  [Function `to_u256`](#0x1_from_bcs_to_u256)
-  [Function `to_address`](#0x1_from_bcs_to_address)
-  [Function `to_bytes`](#0x1_from_bcs_to_bytes)
-  [Function `to_string`](#0x1_from_bcs_to_string)
-  [Function `from_bytes`](#0x1_from_bcs_from_bytes)
-  [Specification](#@Specification_1)
    -  [Function `from_bytes`](#@Specification_1_from_bytes)


<pre><code>use 0x1::string;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_from_bcs_EINVALID_UTF8"></a>

UTF8 check failed in conversion from bytes to string


<pre><code>const EINVALID_UTF8: u64 &#61; 1;
</code></pre>



<a id="0x1_from_bcs_to_bool"></a>

## Function `to_bool`



<pre><code>public fun to_bool(v: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_bool(v: vector&lt;u8&gt;): bool &#123;
    from_bytes&lt;bool&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u8"></a>

## Function `to_u8`



<pre><code>public fun to_u8(v: vector&lt;u8&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u8(v: vector&lt;u8&gt;): u8 &#123;
    from_bytes&lt;u8&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u16"></a>

## Function `to_u16`



<pre><code>public fun to_u16(v: vector&lt;u8&gt;): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u16(v: vector&lt;u8&gt;): u16 &#123;
    from_bytes&lt;u16&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u32"></a>

## Function `to_u32`



<pre><code>public fun to_u32(v: vector&lt;u8&gt;): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u32(v: vector&lt;u8&gt;): u32 &#123;
    from_bytes&lt;u32&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u64"></a>

## Function `to_u64`



<pre><code>public fun to_u64(v: vector&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u64(v: vector&lt;u8&gt;): u64 &#123;
    from_bytes&lt;u64&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u128"></a>

## Function `to_u128`



<pre><code>public fun to_u128(v: vector&lt;u8&gt;): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u128(v: vector&lt;u8&gt;): u128 &#123;
    from_bytes&lt;u128&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_u256"></a>

## Function `to_u256`



<pre><code>public fun to_u256(v: vector&lt;u8&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_u256(v: vector&lt;u8&gt;): u256 &#123;
    from_bytes&lt;u256&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_address"></a>

## Function `to_address`



<pre><code>public fun to_address(v: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_address(v: vector&lt;u8&gt;): address &#123;
    from_bytes&lt;address&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_bytes"></a>

## Function `to_bytes`



<pre><code>public fun to_bytes(v: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_bytes(v: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    from_bytes&lt;vector&lt;u8&gt;&gt;(v)
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_to_string"></a>

## Function `to_string`



<pre><code>public fun to_string(v: vector&lt;u8&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string(v: vector&lt;u8&gt;): String &#123;
    // To make this safe, we need to evaluate the utf8 invariant.
    let s &#61; from_bytes&lt;String&gt;(v);
    assert!(string::internal_check_utf8(string::bytes(&amp;s)), EINVALID_UTF8);
    s
&#125;
</code></pre>



</details>

<a id="0x1_from_bcs_from_bytes"></a>

## Function `from_bytes`

Package private native function to deserialize a type T.

Note that this function does not put any constraint on <code>T</code>. If code uses this function to
deserialize a linear value, its their responsibility that the data they deserialize is
owned.


<pre><code>public(friend) fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) native fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="0x1_from_bcs_deserialize"></a>


<pre><code>fun deserialize&lt;T&gt;(bytes: vector&lt;u8&gt;): T;
<a id="0x1_from_bcs_deserializable"></a>
fun deserializable&lt;T&gt;(bytes: vector&lt;u8&gt;): bool;
axiom&lt;T&gt; forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    ( b1 &#61;&#61; b2 &#61;&#61;&gt; deserializable&lt;T&gt;(b1) &#61;&#61; deserializable&lt;T&gt;(b2) );
axiom&lt;T&gt; forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    ( b1 &#61;&#61; b2 &#61;&#61;&gt; deserialize&lt;T&gt;(b1) &#61;&#61; deserialize&lt;T&gt;(b2) );
</code></pre>



<a id="@Specification_1_from_bytes"></a>

### Function `from_bytes`


<pre><code>public(friend) fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T
</code></pre>




<pre><code>pragma opaque;
aborts_if !deserializable&lt;T&gt;(bytes);
ensures result &#61;&#61; deserialize&lt;T&gt;(bytes);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
