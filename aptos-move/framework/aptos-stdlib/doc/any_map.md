
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
<b>use</b> <a href="ordered_map.md#0x1_ordered_map">0x1::ordered_map</a>;
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
<code>entries: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;<a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
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
        entries: <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_any_map_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_add">add</a>&lt;T: drop, store&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>, x: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_add">add</a>&lt;T: drop + store&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>, x: T) {
    self.entries.<a href="any_map.md#0x1_any_map_add">add</a>(<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;(), to_bytes(&x));
}
</code></pre>



</details>

<a id="0x1_any_map_get_copy"></a>

## Function `get_copy`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_get_copy">get_copy</a>&lt;T: <b>copy</b>, drop, store&gt;(self: &<a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_get_copy">get_copy</a>&lt;T: <b>copy</b> + drop + store&gt;(self: &<a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): T {
    <b>let</b> data = self.entries.borrow(&<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(<a href="../../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(data, 0, <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data)))
}
</code></pre>



</details>

<a id="0x1_any_map_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): T {
    <b>let</b> data = self.entries.<a href="any_map.md#0x1_any_map_remove">remove</a>(&<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(data)
}
</code></pre>



</details>

<a id="0x1_any_map_remove_if_present"></a>

## Function `remove_if_present`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove_if_present">remove_if_present</a>&lt;T&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_remove_if_present">remove_if_present</a>&lt;T&gt;(self: &<b>mut</b> <a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;T&gt; {
    <b>let</b> iter = self.entries.find(&<a href="type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;());
    <b>if</b> (iter.iter_is_end(&self.entries)) {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="from_bcs.md#0x1_from_bcs_from_bytes">from_bcs::from_bytes</a>&lt;T&gt;(iter.iter_remove(&<b>mut</b> self.entries)))
    }
}
</code></pre>



</details>

<a id="0x1_any_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_length">length</a>(self: &<a href="any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="any_map.md#0x1_any_map_length">length</a>(self: &<a href="any_map.md#0x1_any_map_AnyMap">AnyMap</a>): u64 {
    self.entries.<a href="any_map.md#0x1_any_map_length">length</a>()
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
