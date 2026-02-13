
<a id="0x1_decode_bcs"></a>

# Module `0x1::decode_bcs`

This module implements BCS (de)serialization in Move.
Full specification can be found here: https://github.com/diem/bcs


-  [Struct `BCS`](#0x1_decode_bcs_BCS)
-  [Constants](#@Constants_0)
-  [Function `to_bytes`](#0x1_decode_bcs_to_bytes)
-  [Function `new`](#0x1_decode_bcs_new)
-  [Function `into_remainder_bytes`](#0x1_decode_bcs_into_remainder_bytes)
-  [Function `peel_bool`](#0x1_decode_bcs_peel_bool)
-  [Function `peel_u8`](#0x1_decode_bcs_peel_u8)
-  [Function `peel_u16`](#0x1_decode_bcs_peel_u16)
-  [Function `peel_u32`](#0x1_decode_bcs_peel_u32)
-  [Function `peel_u64`](#0x1_decode_bcs_peel_u64)
-  [Function `peel_u128`](#0x1_decode_bcs_peel_u128)
-  [Function `peel_u256`](#0x1_decode_bcs_peel_u256)
-  [Function `peel_vec_length`](#0x1_decode_bcs_peel_vec_length)
-  [Function `peel_vec_bool`](#0x1_decode_bcs_peel_vec_bool)
-  [Function `peel_vec_u8`](#0x1_decode_bcs_peel_vec_u8)
-  [Function `peel_vec_u16`](#0x1_decode_bcs_peel_vec_u16)
-  [Function `peel_vec_u32`](#0x1_decode_bcs_peel_vec_u32)
-  [Function `peel_vec_u64`](#0x1_decode_bcs_peel_vec_u64)
-  [Function `peel_vec_u128`](#0x1_decode_bcs_peel_vec_u128)
-  [Function `peel_vec_u256`](#0x1_decode_bcs_peel_vec_u256)
-  [Function `peel_vec_vec_u8`](#0x1_decode_bcs_peel_vec_vec_u8)
-  [Function `peel_vec_vec_vec_u8`](#0x1_decode_bcs_peel_vec_vec_vec_u8)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_decode_bcs_BCS"></a>

## Struct `BCS`

A helper struct that saves resources on operations. For better
vector performance, it stores reversed bytes of the BCS and
enables use of <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a></code>.


<pre><code><b>struct</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_decode_bcs_ELenOutOfRange"></a>

For when ULEB byte is out of range (or not found).


<pre><code><b>const</b> <a href="decode_bcs.md#0x1_decode_bcs_ELenOutOfRange">ELenOutOfRange</a>: u64 = 2;
</code></pre>



<a id="0x1_decode_bcs_ENotBool"></a>

For when the boolean value different than <code>0</code> or <code>1</code>.


<pre><code><b>const</b> <a href="decode_bcs.md#0x1_decode_bcs_ENotBool">ENotBool</a>: u64 = 1;
</code></pre>



<a id="0x1_decode_bcs_EOutOfRange"></a>

For when bytes length is less than required for deserialization.


<pre><code><b>const</b> <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>: u64 = 0;
</code></pre>



<a id="0x1_decode_bcs_to_bytes"></a>

## Function `to_bytes`

Get BCS serialized bytes for any value.
Re-exports stdlib <code><a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_to_bytes">to_bytes</a>&lt;T&gt;(value: &T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_to_bytes">to_bytes</a>&lt;T&gt;(value: &T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(value)
}
</code></pre>



</details>

<a id="0x1_decode_bcs_new"></a>

## Function `new`

Creates a new instance of BCS wrapper that holds inversed
bytes for better performance.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_new">new</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_new">new</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a> {
    v::reverse(&<b>mut</b> bytes);
    <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a> { bytes }
}
</code></pre>



</details>

<a id="0x1_decode_bcs_into_remainder_bytes"></a>

## Function `into_remainder_bytes`

Unpack the <code><a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a></code> struct returning the leftover bytes.
Useful for passing the data further after partial deserialization.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_into_remainder_bytes">into_remainder_bytes</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_into_remainder_bytes">into_remainder_bytes</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a> { bytes } = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>;
    v::reverse(&<b>mut</b> bytes);
    bytes
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_bool"></a>

## Function `peel_bool`

Read a <code>bool</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_bool">peel_bool</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_bool">peel_bool</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): bool {
    <b>let</b> value = <a href="decode_bcs.md#0x1_decode_bcs_peel_u8">peel_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>);
    <b>if</b> (value == 0) {
        <b>false</b>
    } <b>else</b> <b>if</b> (value == 1) {
        <b>true</b>
    } <b>else</b> {
        <b>abort</b> <a href="decode_bcs.md#0x1_decode_bcs_ENotBool">ENotBool</a>
    }
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u8"></a>

## Function `peel_u8`

Read <code>u8</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u8">peel_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u8">peel_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u8 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 1, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);
    v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes)
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u16"></a>

## Function `peel_u16`

Read <code>u16</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u16">peel_u16</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u16">peel_u16</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u16 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 2, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);
    <b>let</b> (value, i) = (0u16, 0u8);
    <b>while</b> (i &lt; 16) {
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u16);
        value = value | (byte &lt;&lt; i);
        i = i + 8;
    };
    value
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u32"></a>

## Function `peel_u32`

Read <code>u32</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u32">peel_u32</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u32">peel_u32</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u32 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 4, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);
    <b>let</b> (value, i) = (0u32, 0u8);
    <b>while</b> (i &lt; 32) {
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u32);
        value = value | (byte &lt;&lt; i);
        i = i + 8;
    };
    value
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u64"></a>

## Function `peel_u64`

Read <code>u64</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u64">peel_u64</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u64">peel_u64</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u64 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 8, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);
    <b>let</b> (value, i) = (0u64, 0u8);
    <b>while</b> (i &lt; 64) {
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u64);
        value = value | (byte &lt;&lt; i);
        i = i + 8;
    };
    value
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u128"></a>

## Function `peel_u128`

Read <code>u128</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u128">peel_u128</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u128">peel_u128</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u128 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 16, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);

    <b>let</b> (value, i) = (0u128, 0u8);
    <b>while</b> (i &lt; 128) {
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u128);
        value = value | (byte &lt;&lt; i);
        i = i + 8;
    };

    value
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_u256"></a>

## Function `peel_u256`

Read <code>u256</code> value from bcs-serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u256">peel_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_u256">peel_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u256 {
    <b>assert</b>!(v::length(&<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) &gt;= 16, <a href="decode_bcs.md#0x1_decode_bcs_EOutOfRange">EOutOfRange</a>);

    <b>let</b> (value, i) = (0u256, 0u8);
    <b>while</b> (i &lt; 255) {
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u256);
        value = value | (byte &lt;&lt; i);
        i = i + 8;
    };

    value
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_length"></a>

## Function `peel_vec_length`

Read ULEB bytes expecting a vector length. Result should
then be used to perform <code>peel_*</code> operation LEN times.

In BCS <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a></code> length is implemented with ULEB128;
See more here: https://en.wikipedia.org/wiki/LEB128


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): u64 {
    <b>let</b> (total, shift, len) = (0u64, 0, 0);
    <b>while</b> (<b>true</b>) {
        <b>assert</b>!(len &lt;= 4, <a href="decode_bcs.md#0x1_decode_bcs_ELenOutOfRange">ELenOutOfRange</a>);
        <b>let</b> byte = (v::pop_back(&<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>.bytes) <b>as</b> u64);
        len = len + 1;
        total = total | ((byte & 0x7f) &lt;&lt; shift);
        <b>if</b> ((byte & 0x80) == 0) {
            <b>break</b>
        };
        shift = shift + 7;
    };
    total
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_bool"></a>

## Function `peel_vec_bool`

Peel a vector of <code>bool</code> from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_bool">peel_vec_bool</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_bool">peel_vec_bool</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_bool">peel_bool</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u8"></a>

## Function `peel_vec_u8`

Peel a vector of <code>u8</code> (eg string) from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u8">peel_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u8">peel_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u8">peel_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u16"></a>

## Function `peel_vec_u16`

Peel a vector of <code>u16</code> (eg string) from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u16">peel_vec_u16</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u16">peel_vec_u16</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u16">peel_u16</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u32"></a>

## Function `peel_vec_u32`

Peel a vector of <code>u32</code> (eg string) from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u32">peel_vec_u32</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u32&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u32">peel_vec_u32</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u32&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u32">peel_u32</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u64"></a>

## Function `peel_vec_u64`

Peel a vector of <code>u64</code> from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u64">peel_vec_u64</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u64">peel_vec_u64</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u64">peel_u64</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u128"></a>

## Function `peel_vec_u128`

Peel a vector of <code>u128</code> from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u128">peel_vec_u128</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u128">peel_vec_u128</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u128&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u128">peel_u128</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_u256"></a>

## Function `peel_vec_u256`

Peel a vector of <code>u256</code> from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u256">peel_vec_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u256&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u256">peel_vec_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u256&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_u256">peel_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_vec_u8"></a>

## Function `peel_vec_vec_u8`

Peel a <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code> (eg vec of string) from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_vec_u8">peel_vec_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_vec_u8">peel_vec_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_u8">peel_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_decode_bcs_peel_vec_vec_vec_u8"></a>

## Function `peel_vec_vec_vec_u8`

Peel a <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;</code> (eg vec of string) from serialized bytes.


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_vec_vec_u8">peel_vec_vec_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">decode_bcs::BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_vec_vec_u8">peel_vec_vec_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>: &<b>mut</b> <a href="decode_bcs.md#0x1_decode_bcs_BCS">BCS</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>let</b> (len, i, res) = (<a href="decode_bcs.md#0x1_decode_bcs_peel_vec_length">peel_vec_length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>), 0, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <b>while</b> (i &lt; len) {
        v::push_back(&<b>mut</b> res, <a href="decode_bcs.md#0x1_decode_bcs_peel_vec_vec_u8">peel_vec_vec_u8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">bcs</a>));
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
