
<a id="0x1_bcs_stream"></a>

# Module `0x1::bcs_stream`

This module enables the deserialization of BCS-formatted byte arrays into Move primitive types.
Deserialization Strategies:
- Per-Byte Deserialization: Employed for most types to ensure lower gas consumption, this method processes each byte
individually to match the length and type requirements of target Move types.
- Exception: For the <code>deserialize_address</code> function, the function-based approach from <code>aptos_std::from_bcs</code> is used
due to type constraints, even though it is generally more gas-intensive.
- This can be optimized further by introducing native vector slices.
Application:
- This deserializer is particularly valuable for processing BCS serialized data within Move modules,
especially useful for systems requiring cross-chain message interpretation or off-chain data verification.


-  [Struct `BCSStream`](#0x1_bcs_stream_BCSStream)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_bcs_stream_new)
-  [Function `has_remaining`](#0x1_bcs_stream_has_remaining)
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
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_bcs_stream_BCSStream"></a>

## Struct `BCSStream`



<pre><code><b>struct</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_new">new</a>(data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_new">new</a>(data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
    <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
        data,
        cur: 0,
    }
}
</code></pre>



</details>

<a id="0x1_bcs_stream_has_remaining"></a>

## Function `has_remaining`



<pre><code>#[lint::skip(#[needless_mutable_reference])]
<b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_has_remaining">has_remaining</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_has_remaining">has_remaining</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): bool {
    stream.cur &lt; stream.data.length()
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

    <b>while</b> (stream.cur &lt; stream.data.length()) {
        <b>let</b> byte = stream.data[stream.cur];
        stream.cur += 1;

        <b>let</b> val = ((byte & 0x7f) <b>as</b> u64);
        <b>if</b> (((val &lt;&lt; shift) &gt;&gt; shift) != val) {
            <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
        };
        res |= (val &lt;&lt; shift);

        <b>if</b> ((byte & 0x80) == 0) {
            <b>if</b> (shift &gt; 0 && val == 0) {
                <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
            };
            <b>return</b> res
        };

        shift += 7;
        <b>if</b> (shift &gt; 64) {
            <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
        };
    };

    <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>)
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
    <b>assert</b>!(stream.cur &lt; stream.data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> byte = stream.data[stream.cur];
    stream.cur += 1;
    <b>if</b> (byte == 0) {
        <b>false</b>
    } <b>else</b> <b>if</b> (byte == 1) {
        <b>true</b>
    } <b>else</b> {
        <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="bcs_stream.md#0x1_bcs_stream_EMALFORMED_DATA">EMALFORMED_DATA</a>)
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

    <b>assert</b>!(cur + 32 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res = <a href="from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(data.slice(cur, cur + 32));

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

    <b>assert</b>!(cur &lt; data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));

    <b>let</b> res = data[cur];

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

    <b>assert</b>!(cur + 2 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (data[cur] <b>as</b> u16) |
            ((data[cur + 1] <b>as</b> u16) &lt;&lt; 8)
    ;

    stream.cur += 2;
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

    <b>assert</b>!(cur + 4 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (data[cur] <b>as</b> u32) |
            ((data[cur + 1] <b>as</b> u32) &lt;&lt; 8) |
            ((data[cur + 2] <b>as</b> u32) &lt;&lt; 16) |
            ((data[cur + 3] <b>as</b> u32) &lt;&lt; 24)
    ;

    stream.cur += 4;
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

    <b>assert</b>!(cur + 8 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (data[cur] <b>as</b> u64) |
            ((data[cur + 1] <b>as</b> u64) &lt;&lt; 8) |
            ((data[cur + 2] <b>as</b> u64) &lt;&lt; 16) |
            ((data[cur + 3] <b>as</b> u64) &lt;&lt; 24) |
            ((data[cur + 4] <b>as</b> u64) &lt;&lt; 32) |
            ((data[cur + 5] <b>as</b> u64) &lt;&lt; 40) |
            ((data[cur + 6] <b>as</b> u64) &lt;&lt; 48) |
            ((data[cur + 7] <b>as</b> u64) &lt;&lt; 56)
    ;

    stream.cur += 8;
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

    <b>assert</b>!(cur + 16 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (data[cur] <b>as</b> u128) |
            ((data[cur + 1] <b>as</b> u128) &lt;&lt; 8) |
            ((data[cur + 2] <b>as</b> u128) &lt;&lt; 16) |
            ((data[cur + 3] <b>as</b> u128) &lt;&lt; 24) |
            ((data[cur + 4] <b>as</b> u128) &lt;&lt; 32) |
            ((data[cur + 5] <b>as</b> u128) &lt;&lt; 40) |
            ((data[cur + 6] <b>as</b> u128) &lt;&lt; 48) |
            ((data[cur + 7] <b>as</b> u128) &lt;&lt; 56) |
            ((data[cur + 8] <b>as</b> u128) &lt;&lt; 64) |
            ((data[cur + 9] <b>as</b> u128) &lt;&lt; 72) |
            ((data[cur + 10] <b>as</b> u128) &lt;&lt; 80) |
            ((data[cur + 11] <b>as</b> u128) &lt;&lt; 88) |
            ((data[cur + 12] <b>as</b> u128) &lt;&lt; 96) |
            ((data[cur + 13] <b>as</b> u128) &lt;&lt; 104) |
            ((data[cur + 14] <b>as</b> u128) &lt;&lt; 112) |
            ((data[cur + 15] <b>as</b> u128) &lt;&lt; 120)
    ;

    stream.cur += 16;
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

    <b>assert</b>!(cur + 32 &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));
    <b>let</b> res =
        (data[cur] <b>as</b> u256) |
            ((data[cur + 1] <b>as</b> u256) &lt;&lt; 8) |
            ((data[cur + 2] <b>as</b> u256) &lt;&lt; 16) |
            ((data[cur + 3] <b>as</b> u256) &lt;&lt; 24) |
            ((data[cur + 4] <b>as</b> u256) &lt;&lt; 32) |
            ((data[cur + 5] <b>as</b> u256) &lt;&lt; 40) |
            ((data[cur + 6] <b>as</b> u256) &lt;&lt; 48) |
            ((data[cur + 7] <b>as</b> u256) &lt;&lt; 56) |
            ((data[cur + 8] <b>as</b> u256) &lt;&lt; 64) |
            ((data[cur + 9] <b>as</b> u256) &lt;&lt; 72) |
            ((data[cur + 10] <b>as</b> u256) &lt;&lt; 80) |
            ((data[cur + 11] <b>as</b> u256) &lt;&lt; 88) |
            ((data[cur + 12] <b>as</b> u256) &lt;&lt; 96) |
            ((data[cur + 13] <b>as</b> u256) &lt;&lt; 104) |
            ((data[cur + 14] <b>as</b> u256) &lt;&lt; 112) |
            ((data[cur + 15] <b>as</b> u256) &lt;&lt; 120) |
            ((data[cur + 16] <b>as</b> u256) &lt;&lt; 128) |
            ((data[cur + 17] <b>as</b> u256) &lt;&lt; 136) |
            ((data[cur + 18] <b>as</b> u256) &lt;&lt; 144) |
            ((data[cur + 19] <b>as</b> u256) &lt;&lt; 152) |
            ((data[cur + 20] <b>as</b> u256) &lt;&lt; 160) |
            ((data[cur + 21] <b>as</b> u256) &lt;&lt; 168) |
            ((data[cur + 22] <b>as</b> u256) &lt;&lt; 176) |
            ((data[cur + 23] <b>as</b> u256) &lt;&lt; 184) |
            ((data[cur + 24] <b>as</b> u256) &lt;&lt; 192) |
            ((data[cur + 25] <b>as</b> u256) &lt;&lt; 200) |
            ((data[cur + 26] <b>as</b> u256) &lt;&lt; 208) |
            ((data[cur + 27] <b>as</b> u256) &lt;&lt; 216) |
            ((data[cur + 28] <b>as</b> u256) &lt;&lt; 224) |
            ((data[cur + 29] <b>as</b> u256) &lt;&lt; 232) |
            ((data[cur + 30] <b>as</b> u256) &lt;&lt; 240) |
            ((data[cur + 31] <b>as</b> u256) &lt;&lt; 248);

    stream.cur += 32;
    res
}
</code></pre>



</details>

<a id="0x1_bcs_stream_deserialize_u256_entry"></a>

## Function `deserialize_u256_entry`

Deserializes a <code>u256</code> value from the stream.


<pre><code><b>public</b> entry <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256_entry">deserialize_u256_entry</a>(data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cursor: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_u256_entry">deserialize_u256_entry</a>(data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cursor: u64) {
    <b>let</b> stream = <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a> {
        data,
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


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">deserialize_vector</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>|E): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_vector">deserialize_vector</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>| E): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt; {
    <b>let</b> len = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream);
    <b>let</b> v = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    for (i in 0..len) {
        v.push_back(elem_deserializer(stream));
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


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_string">deserialize_string</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_string">deserialize_string</a>(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>): String {
    <b>let</b> len = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_uleb128">deserialize_uleb128</a>(stream);
    <b>let</b> data = &stream.data;
    <b>let</b> cur = stream.cur;

    <b>assert</b>!(cur + len &lt;= data.length(), <a href="../../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="bcs_stream.md#0x1_bcs_stream_EOUT_OF_BYTES">EOUT_OF_BYTES</a>));

    <b>let</b> res = <a href="../../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(data.slice(cur, cur + len));
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


<pre><code><b>public</b> <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_option">deserialize_option</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">bcs_stream::BCSStream</a>|E): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;E&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="bcs_stream.md#0x1_bcs_stream_deserialize_option">deserialize_option</a>&lt;E&gt;(stream: &<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>, elem_deserializer: |&<b>mut</b> <a href="bcs_stream.md#0x1_bcs_stream_BCSStream">BCSStream</a>| E): Option&lt;E&gt; {
    <b>let</b> is_data = <a href="bcs_stream.md#0x1_bcs_stream_deserialize_bool">deserialize_bool</a>(stream);
    <b>if</b> (is_data) {
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(elem_deserializer(stream))
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
