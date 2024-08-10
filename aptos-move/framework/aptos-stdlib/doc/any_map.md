
<a id="0x1_any_map"></a>

# Module `0x1::any_map`



-  [Struct `AnyMap`](#0x1_any_map_AnyMap)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_any_map_new)
-  [Function `add`](#0x1_any_map_add)
-  [Function `get_copy`](#0x1_any_map_get_copy)
-  [Function `remove`](#0x1_any_map_remove)
-  [Function `remove_if_present`](#0x1_any_map_remove_if_present)
-  [Function `length`](#0x1_any_map_length)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_any_map_AnyMap"></a>

## Struct `AnyMap`



<pre><code><b>struct</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_any_map_ETYPE_MISMATCH"></a>

The type provided for <code>unpack</code> is not the same as was given for <code>pack</code>.


<pre><code><b>const</b> <a href="any_map.md#0x1_any_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_any_map_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_new">new</a>(): <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_new">new</a>(): <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a> {
    <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a> {
        entries: <a href="simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_any_map_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_add">add</a>&lt;T: drop, store&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>, x: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_add">add</a>&lt;T: drop + store&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>, x: T) {
    <a href="simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> map.entries, <a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;(), to_bytes(&x));
}
</code></pre>



</details>

<a id="0x1_any_map_get_copy"></a>

## Function `get_copy`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_get_copy">get_copy</a>&lt;T: <b>copy</b>, drop, store&gt;(map: &<a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_get_copy">get_copy</a>&lt;T: <b>copy</b> + drop + store&gt;(map: &<a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): T {
    <b>let</b> data = <a href="simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&map.entries, &<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(<a href="../../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(data, 0, <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data)))
}
</code></pre>



</details>

<a id="0x1_any_map_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove">remove</a>&lt;T&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove">remove</a>&lt;T&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): T {
    <b>let</b> (_key, data) = <a href="simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> map.entries, &<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(data)
}
</code></pre>



</details>

<a id="0x1_any_map_remove_if_present"></a>

## Function `remove_if_present`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove_if_present">remove_if_present</a>&lt;T&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove_if_present">remove_if_present</a>&lt;T&gt;(map: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;T&gt; {
    <b>let</b> data = <a href="simple_map.md#0x1_simple_map_remove_if_present">simple_map::remove_if_present</a>(&<b>mut</b> map.entries, &<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&data)) {
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(data)))
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_any_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_length">length</a>(map: &<a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_length">length</a>(map: &<a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): u64 {
    <a href="simple_map.md#0x1_simple_map_length">simple_map::length</a>(&map.entries)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
