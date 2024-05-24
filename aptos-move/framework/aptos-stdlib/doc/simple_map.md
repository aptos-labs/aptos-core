
<a id="0x1_simple_map"></a>

# Module `0x1::simple_map`

This module provides a solution for unsorted maps, that is it has the properties that
1) Keys point to Values
2) Each Key must be unique
3) A Key can be found within O(N) time
4) The keys are unsorted.
5) Adds and removals take O(N) time


-  [Struct `SimpleMap`](#0x1_simple_map_SimpleMap)
-  [Struct `Element`](#0x1_simple_map_Element)
-  [Constants](#@Constants_0)
-  [Function `length`](#0x1_simple_map_length)
-  [Function `new`](#0x1_simple_map_new)
-  [Function `new_from`](#0x1_simple_map_new_from)
-  [Function `create`](#0x1_simple_map_create)
-  [Function `borrow`](#0x1_simple_map_borrow)
-  [Function `borrow_mut`](#0x1_simple_map_borrow_mut)
-  [Function `contains_key`](#0x1_simple_map_contains_key)
-  [Function `destroy_empty`](#0x1_simple_map_destroy_empty)
-  [Function `add`](#0x1_simple_map_add)
-  [Function `add_all`](#0x1_simple_map_add_all)
-  [Function `upsert`](#0x1_simple_map_upsert)
-  [Function `keys`](#0x1_simple_map_keys)
-  [Function `values`](#0x1_simple_map_values)
-  [Function `to_vec_pair`](#0x1_simple_map_to_vec_pair)
-  [Function `destroy`](#0x1_simple_map_destroy)
-  [Function `remove`](#0x1_simple_map_remove)
-  [Function `find`](#0x1_simple_map_find)
-  [Specification](#@Specification_1)
    -  [Struct `SimpleMap`](#@Specification_1_SimpleMap)
    -  [Function `length`](#@Specification_1_length)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `new_from`](#@Specification_1_new_from)
    -  [Function `create`](#@Specification_1_create)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `borrow_mut`](#@Specification_1_borrow_mut)
    -  [Function `contains_key`](#@Specification_1_contains_key)
    -  [Function `destroy_empty`](#@Specification_1_destroy_empty)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `add_all`](#@Specification_1_add_all)
    -  [Function `upsert`](#@Specification_1_upsert)
    -  [Function `keys`](#@Specification_1_keys)
    -  [Function `values`](#@Specification_1_values)
    -  [Function `to_vec_pair`](#@Specification_1_to_vec_pair)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `find`](#@Specification_1_find)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_simple_map_SimpleMap"></a>

## Struct `SimpleMap`



<pre><code><b>struct</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt; <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="simple_map.md#0x1_simple_map_Element">simple_map::Element</a>&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_simple_map_Element"></a>

## Struct `Element`



<pre><code><b>struct</b> <a href="simple_map.md#0x1_simple_map_Element">Element</a>&lt;Key, Value&gt; <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: Key</code>
</dt>
<dd>

</dd>
<dt>
<code>value: Value</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_simple_map_EKEY_ALREADY_EXISTS"></a>

Map key already exists


<pre><code><b>const</b> <a href="simple_map.md#0x1_simple_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_simple_map_EKEY_NOT_FOUND"></a>

Map key is not found


<pre><code><b>const</b> <a href="simple_map.md#0x1_simple_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_simple_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_length">length</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_length">length</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;): u64 &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;map.data)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_new"></a>

## Function `new`

Create an empty SimpleMap.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new">new</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new">new</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt; &#123;<br />    <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a> &#123;<br />        data: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_new_from"></a>

## Function `new_from`

Create a SimpleMap from a vector of keys and values. The keys must be unique.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new_from">new_from</a>&lt;Key: store, Value: store&gt;(keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new_from">new_from</a>&lt;Key: store, Value: store&gt;(<br />    keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;,<br />    values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;,<br />): <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt; &#123;<br />    <b>let</b> map &#61; <a href="simple_map.md#0x1_simple_map_new">new</a>();<br />    <a href="simple_map.md#0x1_simple_map_add_all">add_all</a>(&amp;<b>mut</b> map, keys, values);<br />    map<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_create"></a>

## Function `create`

Create an empty SimpleMap.
This function is deprecated, use <code>new</code> instead.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_create">create</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_create">create</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt; &#123;<br />    <a href="simple_map.md#0x1_simple_map_new">new</a>()<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow">borrow</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): &amp;Value<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow">borrow</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: &amp;Key,<br />): &amp;Value &#123;<br />    <b>let</b> maybe_idx &#61; <a href="simple_map.md#0x1_simple_map_find">find</a>(map, key);<br />    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_idx), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="simple_map.md#0x1_simple_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));<br />    <b>let</b> idx &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maybe_idx);<br />    &amp;<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;map.data, idx).value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow_mut">borrow_mut</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): &amp;<b>mut</b> Value<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow_mut">borrow_mut</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: &amp;Key,<br />): &amp;<b>mut</b> Value &#123;<br />    <b>let</b> maybe_idx &#61; <a href="simple_map.md#0x1_simple_map_find">find</a>(map, key);<br />    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_idx), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="simple_map.md#0x1_simple_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));<br />    <b>let</b> idx &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maybe_idx);<br />    &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> map.data, idx).value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_contains_key">contains_key</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_contains_key">contains_key</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: &amp;Key,<br />): bool &#123;<br />    <b>let</b> maybe_idx &#61; <a href="simple_map.md#0x1_simple_map_find">find</a>(map, key);<br />    <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_idx)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_destroy_empty"></a>

## Function `destroy_empty`



<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_destroy_empty">destroy_empty</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_destroy_empty">destroy_empty</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;) &#123;<br />    <b>let</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a> &#123; data &#125; &#61; map;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(data);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_add"></a>

## Function `add`

Add a key/value pair to the map. The key must not already exist.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add">add</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: Key, value: Value)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add">add</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: Key,<br />    value: Value,<br />) &#123;<br />    <b>let</b> maybe_idx &#61; <a href="simple_map.md#0x1_simple_map_find">find</a>(map, &amp;key);<br />    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&amp;maybe_idx), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="simple_map.md#0x1_simple_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));<br /><br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> map.data, <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the map. The keys must not already exist.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add_all">add_all</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add_all">add_all</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;,<br />    values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;,<br />) &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_zip">vector::zip</a>(keys, values, &#124;key, value&#124; &#123;<br />        <a href="simple_map.md#0x1_simple_map_add">add</a>(map, key, value);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_upsert"></a>

## Function `upsert`

Insert key/value pair or update an existing key to a new value


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_upsert">upsert</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: Key, value: Value): (<a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Key&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Value&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_upsert">upsert</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: Key,<br />    value: Value<br />): (std::option::Option&lt;Key&gt;, std::option::Option&lt;Value&gt;) &#123;<br />    <b>let</b> data &#61; &amp;<b>mut</b> map.data;<br />    <b>let</b> len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(data);<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>let</b> element &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(data, i);<br />        <b>if</b> (&amp;element.key &#61;&#61; &amp;key) &#123;<br />            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(data, <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125;);<br />            <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(data, i, len);<br />            <b>let</b> <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125; &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(data);<br />            <b>return</b> (std::option::some(key), std::option::some(value))<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> map.data, <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125;);<br />    (std::option::none(), std::option::none())<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_keys"></a>

## Function `keys`

Return all keys in the map. This requires keys to be copyable.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_keys">keys</a>&lt;Key: <b>copy</b>, Value&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_keys">keys</a>&lt;Key: <b>copy</b>, Value&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt; &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&amp;map.data, &#124;e&#124; &#123;<br />        <b>let</b> e: &amp;<a href="simple_map.md#0x1_simple_map_Element">Element</a>&lt;Key, Value&gt; &#61; e;<br />        e.key<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_values"></a>

## Function `values`

Return all values in the map. This requires values to be copyable.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_values">values</a>&lt;Key, Value: <b>copy</b>&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_values">values</a>&lt;Key, Value: <b>copy</b>&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt; &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&amp;map.data, &#124;e&#124; &#123;<br />        <b>let</b> e: &amp;<a href="simple_map.md#0x1_simple_map_Element">Element</a>&lt;Key, Value&gt; &#61; e;<br />        e.value<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_to_vec_pair"></a>

## Function `to_vec_pair`

Transform the map into two vectors with the keys and values respectively
Primarily used to destroy a map


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_to_vec_pair">to_vec_pair</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_to_vec_pair">to_vec_pair</a>&lt;Key: store, Value: store&gt;(<br />    map: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;) &#123;<br />    <b>let</b> keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt; &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt; &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a> &#123; data &#125; &#61; map;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(data, &#124;e&#124; &#123;<br />        <b>let</b> <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125; &#61; e;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> keys, key);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> values, value);<br />    &#125;);<br />    (keys, values)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_destroy"></a>

## Function `destroy`

For maps that cannot be dropped this is a utility to destroy them
using lambdas to destroy the individual keys and values.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_destroy">destroy</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, dk: &#124;Key&#124;, dv: &#124;Value&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="simple_map.md#0x1_simple_map_destroy">destroy</a>&lt;Key: store, Value: store&gt;(<br />    map: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    dk: &#124;Key&#124;,<br />    dv: &#124;Value&#124;<br />) &#123;<br />    <b>let</b> (keys, values) &#61; <a href="simple_map.md#0x1_simple_map_to_vec_pair">to_vec_pair</a>(map);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy">vector::destroy</a>(keys, &#124;_k&#124; dk(_k));<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy">vector::destroy</a>(values, &#124;_v&#124; dv(_v));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_remove"></a>

## Function `remove`

Remove a key/value pair from the map. The key must exist.


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_remove">remove</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_remove">remove</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: &amp;Key,<br />): (Key, Value) &#123;<br />    <b>let</b> maybe_idx &#61; <a href="simple_map.md#0x1_simple_map_find">find</a>(map, key);<br />    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_idx), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="simple_map.md#0x1_simple_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));<br />    <b>let</b> placement &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> maybe_idx);<br />    <b>let</b> <a href="simple_map.md#0x1_simple_map_Element">Element</a> &#123; key, value &#125; &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&amp;<b>mut</b> map.data, placement);<br />    (key, value)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_simple_map_find"></a>

## Function `find`



<pre><code><b>fun</b> <a href="simple_map.md#0x1_simple_map_find">find</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="simple_map.md#0x1_simple_map_find">find</a>&lt;Key: store, Value: store&gt;(<br />    map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt;,<br />    key: &amp;Key,<br />): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt; &#123;<br />    <b>let</b> leng &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;map.data);<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; leng) &#123;<br />        <b>let</b> element &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;map.data, i);<br />        <b>if</b> (&amp;element.key &#61;&#61; key) &#123;<br />            <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(i)<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u64&gt;()<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SimpleMap"></a>

### Struct `SimpleMap`


<pre><code><b>struct</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;Key, Value&gt; <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="simple_map.md#0x1_simple_map_Element">simple_map::Element</a>&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>pragma</b> intrinsic &#61; map,<br />    map_new &#61; create,<br />    map_len &#61; length,<br />    map_destroy_empty &#61; destroy_empty,<br />    map_has_key &#61; contains_key,<br />    map_add_no_override &#61; add,<br />    map_del_return_key &#61; remove,<br />    map_borrow &#61; borrow,<br />    map_borrow_mut &#61; borrow_mut,<br />    map_spec_get &#61; spec_get,<br />    map_spec_set &#61; spec_set,<br />    map_spec_del &#61; spec_remove,<br />    map_spec_len &#61; spec_len,<br />    map_spec_has_key &#61; spec_contains_key;<br /></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_length">length</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): u64<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new">new</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_len">spec_len</a>(result) &#61;&#61; 0;<br /><b>ensures</b> [abstract] <b>forall</b> k: Key: !<a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(result, k);<br /></code></pre>



<a id="@Specification_1_new_from"></a>

### Function `new_from`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_new_from">new_from</a>&lt;Key: store, Value: store&gt;(keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_len">spec_len</a>(result) &#61;&#61; len(keys);<br /><b>ensures</b> [abstract] <b>forall</b> k: Key: <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(result, k) &lt;&#61;&#61;&gt; <a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(keys, k);<br /><b>ensures</b> [abstract] <b>forall</b> i in 0..len(keys):<br />    <a href="simple_map.md#0x1_simple_map_spec_get">spec_get</a>(result, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(keys, i)) &#61;&#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(values, i);<br /></code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_create">create</a>&lt;Key: store, Value: store&gt;(): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow">borrow</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): &amp;Value<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_borrow_mut">borrow_mut</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): &amp;<b>mut</b> Value<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_contains_key">contains_key</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): bool<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_destroy_empty">destroy_empty</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add">add</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: Key, value: Value)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_add_all">add_all</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_upsert">upsert</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: Key, value: Value): (<a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Key&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Value&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] !<a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(map), key) &#61;&#61;&gt; <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(result_1);<br /><b>ensures</b> [abstract] !<a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(map), key) &#61;&#61;&gt; <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(result_2);<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(map, key);<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_get">spec_get</a>(map, key) &#61;&#61; value;<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(map), key) &#61;&#61;&gt; ((<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result_1)) &amp;&amp; (<a href="../../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result_1) &#61;&#61; key));<br /><b>ensures</b> [abstract] <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(map), key) &#61;&#61;&gt; ((<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result_2)) &amp;&amp; (<a href="../../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result_2) &#61;&#61; <a href="simple_map.md#0x1_simple_map_spec_get">spec_get</a>(<b>old</b>(map), key)));<br /></code></pre>




<a id="0x1_simple_map_spec_len"></a>


<pre><code><b>native</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_spec_len">spec_len</a>&lt;K, V&gt;(t: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;): num;<br /></code></pre>




<a id="0x1_simple_map_spec_contains_key"></a>


<pre><code><b>native</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>&lt;K, V&gt;(t: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;, k: K): bool;<br /></code></pre>




<a id="0x1_simple_map_spec_set"></a>


<pre><code><b>native</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_spec_set">spec_set</a>&lt;K, V&gt;(t: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;, k: K, v: V): <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;;<br /></code></pre>




<a id="0x1_simple_map_spec_remove"></a>


<pre><code><b>native</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_spec_remove">spec_remove</a>&lt;K, V&gt;(t: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;, k: K): <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;;<br /></code></pre>




<a id="0x1_simple_map_spec_get"></a>


<pre><code><b>native</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_spec_get">spec_get</a>&lt;K, V&gt;(t: <a href="simple_map.md#0x1_simple_map_SimpleMap">SimpleMap</a>&lt;K, V&gt;, k: K): V;<br /></code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_keys">keys</a>&lt;Key: <b>copy</b>, Value&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_values">values</a>&lt;Key, Value: <b>copy</b>&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>



<a id="@Specification_1_to_vec_pair"></a>

### Function `to_vec_pair`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_to_vec_pair">to_vec_pair</a>&lt;Key: store, Value: store&gt;(map: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Value&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /><b>pragma</b> opaque;<br /><b>ensures</b> [abstract]<br />    <b>forall</b> k: Key: <a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(result_1, k) &lt;&#61;&#61;&gt;<br />        <a href="simple_map.md#0x1_simple_map_spec_contains_key">spec_contains_key</a>(map, k);<br /><b>ensures</b> [abstract] <b>forall</b> i in 0..len(result_1):<br />    <a href="simple_map.md#0x1_simple_map_spec_get">spec_get</a>(map, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(result_1, i)) &#61;&#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(result_2, i);<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="simple_map.md#0x1_simple_map_remove">remove</a>&lt;Key: store, Value: store&gt;(map: &amp;<b>mut</b> <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_find"></a>

### Function `find`


<pre><code><b>fun</b> <a href="simple_map.md#0x1_simple_map_find">find</a>&lt;Key: store, Value: store&gt;(map: &amp;<a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;Key, Value&gt;, key: &amp;Key): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
