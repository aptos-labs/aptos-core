
<a id="0x1_bcs_stream"></a>

# Module `0x1::bcs_stream`



-  [Struct `BCSStream`](#0x1_bcs_stream_BCSStream)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_bcs_stream_new)
-  [Function `deserialize_uleb128`](#0x1_bcs_stream_deserialize_uleb128)
-  [Function `deserialize_bool`](#0x1_bcs_stream_deserialize_bool)
-  [Function `deserialize_address`](#0x1_bcs_stream_deserialize_address)
-  [Function `deserialize_u8`](#0x1_bcs_stream_deserialize_u8)
-  [Function `deserialize_u16`](#0x1_bcs_stream_deserialize_u16)
-  [Function `deserialize_u32`](#0x1_bcs_stream_deserialize_u32)
-  [Function `deserialize_u64`](#0x1_bcs_stream_deserialize_u64)
-  [Function `deserialize_u128`](#0x1_bcs_stream_deserialize_u128)
-  [Function `deserialize_u256`](#0x1_bcs_stream_deserialize_u256)
-  [Function `deserialize_u256_entry`](#0x1_bcs_stream_deserialize_u256_entry)
-  [Function `deserialize_vector`](#0x1_bcs_stream_deserialize_vector)
-  [Function `deserialize_string`](#0x1_bcs_stream_deserialize_string)
-  [Function `deserialize_option`](#0x1_bcs_stream_deserialize_option)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_bcs_stream_BCSStream"></a>

## Struct `BCSStream`



<pre><code><b>struct</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Byte buffer containing the serialized data.
</dd>
<dt>
<code>cur: u64</code>
</dt>
<dd>
 Cursor indicating the current position in the byte buffer.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_bcs_stream_EMALFORMED_DATA"></a>

The data does not fit the expected format.


<pre><code><b>const</b> <a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>: u64 = 1;
</code></pre>



<a id="0x1_bcs_stream_EOUT_OF_BYTES"></a>

There are not enough bytes to deserialize for the given type.


<pre><code><b>const</b> <a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>: u64 = 2;
</code></pre>



<a id="0x1_bcs_stream_new"></a>

## Function `new`

Constructs a new BCSStream instance from the provided byte array.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_new">new</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_new">new</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
    <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
        data,
        cur: 0,
    }
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_uleb128"></a>

## Function `deserialize_uleb128`

Deserializes a ULEB128-encoded integer from the stream.
In the BCS format, lengths of vectors are represented using ULEB128 encoding.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u64 {
    <b>let</b> res = 0;
    <b>let</b> shift = 0;

    <b>while</b> (stream.cur &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&stream.data)) {
        <b>let</b> byte = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&stream.data, stream.cur);
        stream.cur = stream.cur + 1;

        <b>let</b> val = ((byte & 0x7f) <b>as</b> u64);
        <b>if</b> (((val &lt;&lt; shift) &gt;&gt; shift) != val) {
            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
        };
        res = res | (val &lt;&lt; shift);

        <b>if</b> ((byte & 0x80) == 0) {
            <b>if</b> (shift &gt; 0 && val == 0) {
                <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
            };
            <b>return</b> res
        };

        shift = shift + 7;
        <b>if</b> (shift &gt; 64) {
            <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
        };
    };

    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>)
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_bool"></a>

## Function `deserialize_bool`

Deserializes a <code>bool</code> value from the stream.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_bool">deserialize_bool</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_bool">deserialize_bool</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): bool {
    <b>assert</b>!(stream.cur &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&stream.data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> byte = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&stream.data, stream.cur);
    stream.cur = stream.cur + 1;
    <b>if</b> (byte == 0) {
        <b>false</b>
    } <b>else</b> <b>if</b> (byte == 1) {
        <b>true</b>
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
    }
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_address"></a>

## Function `deserialize_address`

Deserializes an <code><b>address</b></code> value from the stream.
32-byte <code><b>address</b></code> values are serialized using little-endian byte order.
This function utilizes the <code>to_address</code> function from the <code>aptos_std::from_bcs</code> module,
because the Move type system does not permit per-byte referencing of addresses.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_address">deserialize_address</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_address">deserialize_address</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): <b>address</b> {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 32 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(data, cur, cur + 32));

    stream.cur = cur + 32;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u8"></a>

## Function `deserialize_u8`

Deserializes a <code>u8</code> value from the stream.
1-byte <code>u8</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u8">deserialize_u8</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u8">deserialize_u8</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u8 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));

    <b>let</b> res = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur);

    stream.cur = cur + 1;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u16"></a>

## Function `deserialize_u16`

Deserializes a <code>u16</code> value from the stream.
2-byte <code>u16</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u16">deserialize_u16</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u16">deserialize_u16</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u16 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 2 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur) <b>as</b> u16) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 1) <b>as</b> u16) &lt;&lt; 8)
    ;

    stream.cur = stream.cur + 2;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u32"></a>

## Function `deserialize_u32`

Deserializes a <code>u32</code> value from the stream.
4-byte <code>u32</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u32">deserialize_u32</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u32">deserialize_u32</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u32 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 4 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur) <b>as</b> u32) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 1) <b>as</b> u32) &lt;&lt; 8) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 2) <b>as</b> u32) &lt;&lt; 16) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 3) <b>as</b> u32) &lt;&lt; 24)
    ;

    stream.cur = stream.cur + 4;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u64"></a>

## Function `deserialize_u64`

Deserializes a <code>u64</code> value from the stream.
8-byte <code>u64</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u64">deserialize_u64</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u64">deserialize_u64</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u64 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 8 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur) <b>as</b> u64) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 1) <b>as</b> u64) &lt;&lt; 8) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 2) <b>as</b> u64) &lt;&lt; 16) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 3) <b>as</b> u64) &lt;&lt; 24) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 4) <b>as</b> u64) &lt;&lt; 32) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 5) <b>as</b> u64) &lt;&lt; 40) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 6) <b>as</b> u64) &lt;&lt; 48) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 7) <b>as</b> u64) &lt;&lt; 56)
    ;

    stream.cur = stream.cur + 8;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u128"></a>

## Function `deserialize_u128`

Deserializes a <code>u128</code> value from the stream.
16-byte <code>u128</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u128">deserialize_u128</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u128">deserialize_u128</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u128 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 16 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur) <b>as</b> u128) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 1) <b>as</b> u128) &lt;&lt; 8) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 2) <b>as</b> u128) &lt;&lt; 16) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 3) <b>as</b> u128) &lt;&lt; 24) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 4) <b>as</b> u128) &lt;&lt; 32) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 5) <b>as</b> u128) &lt;&lt; 40) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 6) <b>as</b> u128) &lt;&lt; 48) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 7) <b>as</b> u128) &lt;&lt; 56) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 8) <b>as</b> u128) &lt;&lt; 64) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 9) <b>as</b> u128) &lt;&lt; 72) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 10) <b>as</b> u128) &lt;&lt; 80) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 11) <b>as</b> u128) &lt;&lt; 88) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 12) <b>as</b> u128) &lt;&lt; 96) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 13) <b>as</b> u128) &lt;&lt; 104) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 14) <b>as</b> u128) &lt;&lt; 112) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 15) <b>as</b> u128) &lt;&lt; 120)
    ;

    stream.cur = stream.cur + 16;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u256"></a>

## Function `deserialize_u256`

Deserializes a <code>u256</code> value from the stream.
32-byte <code>u256</code> values are serialized using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256">deserialize_u256</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256">deserialize_u256</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): u256 {
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + 32 &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur) <b>as</b> u256) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 1) <b>as</b> u256) &lt;&lt; 8) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 2) <b>as</b> u256) &lt;&lt; 16) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 3) <b>as</b> u256) &lt;&lt; 24) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 4) <b>as</b> u256) &lt;&lt; 32) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 5) <b>as</b> u256) &lt;&lt; 40) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 6) <b>as</b> u256) &lt;&lt; 48) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 7) <b>as</b> u256) &lt;&lt; 56) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 8) <b>as</b> u256) &lt;&lt; 64) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 9) <b>as</b> u256) &lt;&lt; 72) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 10) <b>as</b> u256) &lt;&lt; 80) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 11) <b>as</b> u256) &lt;&lt; 88) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 12) <b>as</b> u256) &lt;&lt; 96) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 13) <b>as</b> u256) &lt;&lt; 104) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 14) <b>as</b> u256) &lt;&lt; 112) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 15) <b>as</b> u256) &lt;&lt; 120) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 16) <b>as</b> u256) &lt;&lt; 128) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 17) <b>as</b> u256) &lt;&lt; 136) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 18) <b>as</b> u256) &lt;&lt; 144) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 19) <b>as</b> u256) &lt;&lt; 152) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 20) <b>as</b> u256) &lt;&lt; 160) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 21) <b>as</b> u256) &lt;&lt; 168) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 22) <b>as</b> u256) &lt;&lt; 176) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 23) <b>as</b> u256) &lt;&lt; 184) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 24) <b>as</b> u256) &lt;&lt; 192) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 25) <b>as</b> u256) &lt;&lt; 200) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 26) <b>as</b> u256) &lt;&lt; 208) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 27) <b>as</b> u256) &lt;&lt; 216) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 28) <b>as</b> u256) &lt;&lt; 224) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 29) <b>as</b> u256) &lt;&lt; 232) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 30) <b>as</b> u256) &lt;&lt; 240) |
            ((*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, cur + 31) <b>as</b> u256) &lt;&lt; 248)
    ;

    stream.cur = stream.cur + 32;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u256_entry"></a>

## Function `deserialize_u256_entry`

Deserializes a <code>u256</code> value from the stream.


<pre><code><b>public</b> entry <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256_entry">deserialize_u256_entry</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cursor: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256_entry">deserialize_u256_entry</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cursor: u64) {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
        data: data,
        cur: cursor,
    };
    <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256">deserialize_u256</a>(&<b>mut</b> stream);
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_vector"></a>

## Function `deserialize_vector`

Deserializes an array of BCS deserializable elements from the stream.
First, reads the length of the vector, which is in uleb128 format.
After determining the length, it then reads the contents of the vector.
The <code>elem_deserializer</code> lambda expression is used sequentially to deserialize each element of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">deserialize_vector</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>|E): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">deserialize_vector</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>| E): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt; {
    <b>let</b> len = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream);
    <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, elem_deserializer(stream));
        i = i + 1;
    };

    v
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_string"></a>

## Function `deserialize_string`

Deserializes utf-8 <code>String</code> from the stream.
First, reads the length of the String, which is in uleb128 format.
After determining the length, it then reads the contents of the String.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_string">deserialize_string</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_string">deserialize_string</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): String {
    <b>let</b> len = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream);
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + len &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));

    <b>let</b> res = <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(data, cur, cur + len));
    stream.cur = cur + len;

    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_option"></a>

## Function `deserialize_option`

Deserializes <code>Option</code> from the stream.
First, reads a single byte representing the presence (0x01) or absence (0x00) of data.
After determining the presence of data, it then reads the actual data if present.
The <code>elem_deserializer</code> lambda expression is used to deserialize the element contained within the <code>Option</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_option">deserialize_option</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>|E): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;E&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_option">deserialize_option</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>| E): Option&lt;E&gt; {
    <b>let</b> is_data = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_bool">deserialize_bool</a>(stream);
    <b>if</b> (is_data) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(elem_deserializer(stream))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
