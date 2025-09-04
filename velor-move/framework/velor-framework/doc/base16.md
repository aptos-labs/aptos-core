
<a id="0x1_base16"></a>

# Module `0x1::base16`



-  [Function `hex_char_to_u8`](#0x1_base16_hex_char_to_u8)
-  [Function `base16_utf8_to_vec_u8`](#0x1_base16_base16_utf8_to_vec_u8)
-  [Specification](#@Specification_0)
    -  [Function `base16_utf8_to_vec_u8`](#@Specification_0_base16_utf8_to_vec_u8)


<pre><code></code></pre>



<a id="0x1_base16_hex_char_to_u8"></a>

## Function `hex_char_to_u8`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="base16.md#0x1_base16_hex_char_to_u8">hex_char_to_u8</a>(c: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="base16.md#0x1_base16_hex_char_to_u8">hex_char_to_u8</a>(c: u8): u8 {
    <b>if</b> (c &gt;= 48 && c &lt;= 57) {  // '0' <b>to</b> '9'
        c - 48
    } <b>else</b> <b>if</b> (c &gt;= 65 && c &lt;= 70) { // 'A' <b>to</b> 'F'
        c - 55
    } <b>else</b> <b>if</b> (c &gt;= 97 && c &lt;= 102) { // 'a' <b>to</b> 'f'
        c - 87
    } <b>else</b> {
        <b>abort</b> 1
    }
}
</code></pre>



</details>

<a id="0x1_base16_base16_utf8_to_vec_u8"></a>

## Function `base16_utf8_to_vec_u8`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="base16.md#0x1_base16_base16_utf8_to_vec_u8">base16_utf8_to_vec_u8</a>(str: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="base16.md#0x1_base16_base16_utf8_to_vec_u8">base16_utf8_to_vec_u8</a>(str: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> result = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&str)) {
        <b>let</b> c1 = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&str, i);
        <b>let</b> c2 = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&str, i + 1);
        <b>let</b> byte = <a href="base16.md#0x1_base16_hex_char_to_u8">hex_char_to_u8</a>(*c1) &lt;&lt; 4 | <a href="base16.md#0x1_base16_hex_char_to_u8">hex_char_to_u8</a>(*c2);
        <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, byte);
        i = i + 2;
    };
    result
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_base16_utf8_to_vec_u8"></a>

### Function `base16_utf8_to_vec_u8`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="base16.md#0x1_base16_base16_utf8_to_vec_u8">base16_utf8_to_vec_u8</a>(str: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="base16.md#0x1_base16_spec_base16_utf8_to_vec_u8">spec_base16_utf8_to_vec_u8</a>(str);
</code></pre>




<a id="0x1_base16_spec_base16_utf8_to_vec_u8"></a>


<pre><code><b>fun</b> <a href="base16.md#0x1_base16_spec_base16_utf8_to_vec_u8">spec_base16_utf8_to_vec_u8</a>(str: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
