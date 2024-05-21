
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


<pre><code>use 0x1::error;<br/>use 0x1::option;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_simple_map_SimpleMap"></a>

## Struct `SimpleMap`



<pre><code>struct SimpleMap&lt;Key, Value&gt; has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: vector&lt;simple_map::Element&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_simple_map_Element"></a>

## Struct `Element`



<pre><code>struct Element&lt;Key, Value&gt; has copy, drop, store<br/></code></pre>



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


<pre><code>const EKEY_ALREADY_EXISTS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_simple_map_EKEY_NOT_FOUND"></a>

Map key is not found


<pre><code>const EKEY_NOT_FOUND: u64 &#61; 2;<br/></code></pre>



<a id="0x1_simple_map_length"></a>

## Function `length`



<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): u64 &#123;<br/>    vector::length(&amp;map.data)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_new"></a>

## Function `new`

Create an empty SimpleMap.


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): SimpleMap&lt;Key, Value&gt; &#123;<br/>    SimpleMap &#123;<br/>        data: vector::empty(),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_new_from"></a>

## Function `new_from`

Create a SimpleMap from a vector of keys and values. The keys must be unique.


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(<br/>    keys: vector&lt;Key&gt;,<br/>    values: vector&lt;Value&gt;,<br/>): SimpleMap&lt;Key, Value&gt; &#123;<br/>    let map &#61; new();<br/>    add_all(&amp;mut map, keys, values);<br/>    map<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_create"></a>

## Function `create`

Create an empty SimpleMap.
This function is deprecated, use <code>new</code> instead.


<pre><code>&#35;[deprecated]<br/>public fun create&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create&lt;Key: store, Value: store&gt;(): SimpleMap&lt;Key, Value&gt; &#123;<br/>    new()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_borrow"></a>

## Function `borrow`



<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;Value<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(<br/>    map: &amp;SimpleMap&lt;Key, Value&gt;,<br/>    key: &amp;Key,<br/>): &amp;Value &#123;<br/>    let maybe_idx &#61; find(map, key);<br/>    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));<br/>    let idx &#61; option::extract(&amp;mut maybe_idx);<br/>    &amp;vector::borrow(&amp;map.data, idx).value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_borrow_mut"></a>

## Function `borrow_mut`



<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;mut Value<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(<br/>    map: &amp;mut SimpleMap&lt;Key, Value&gt;,<br/>    key: &amp;Key,<br/>): &amp;mut Value &#123;<br/>    let maybe_idx &#61; find(map, key);<br/>    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));<br/>    let idx &#61; option::extract(&amp;mut maybe_idx);<br/>    &amp;mut vector::borrow_mut(&amp;mut map.data, idx).value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_contains_key"></a>

## Function `contains_key`



<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(<br/>    map: &amp;SimpleMap&lt;Key, Value&gt;,<br/>    key: &amp;Key,<br/>): bool &#123;<br/>    let maybe_idx &#61; find(map, key);<br/>    option::is_some(&amp;maybe_idx)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_destroy_empty"></a>

## Function `destroy_empty`



<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: SimpleMap&lt;Key, Value&gt;) &#123;<br/>    let SimpleMap &#123; data &#125; &#61; map;<br/>    vector::destroy_empty(data);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_add"></a>

## Function `add`

Add a key/value pair to the map. The key must not already exist.


<pre><code>public fun add&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;Key: store, Value: store&gt;(<br/>    map: &amp;mut SimpleMap&lt;Key, Value&gt;,<br/>    key: Key,<br/>    value: Value,<br/>) &#123;<br/>    let maybe_idx &#61; find(map, &amp;key);<br/>    assert!(option::is_none(&amp;maybe_idx), error::invalid_argument(EKEY_ALREADY_EXISTS));<br/><br/>    vector::push_back(&amp;mut map.data, Element &#123; key, value &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the map. The keys must not already exist.


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(<br/>    map: &amp;mut SimpleMap&lt;Key, Value&gt;,<br/>    keys: vector&lt;Key&gt;,<br/>    values: vector&lt;Value&gt;,<br/>) &#123;<br/>    vector::zip(keys, values, &#124;key, value&#124; &#123;<br/>        add(map, key, value);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_upsert"></a>

## Function `upsert`

Insert key/value pair or update an existing key to a new value


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value): (option::Option&lt;Key&gt;, option::Option&lt;Value&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(<br/>    map: &amp;mut SimpleMap&lt;Key, Value&gt;,<br/>    key: Key,<br/>    value: Value<br/>): (std::option::Option&lt;Key&gt;, std::option::Option&lt;Value&gt;) &#123;<br/>    let data &#61; &amp;mut map.data;<br/>    let len &#61; vector::length(data);<br/>    let i &#61; 0;<br/>    while (i &lt; len) &#123;<br/>        let element &#61; vector::borrow(data, i);<br/>        if (&amp;element.key &#61;&#61; &amp;key) &#123;<br/>            vector::push_back(data, Element &#123; key, value &#125;);<br/>            vector::swap(data, i, len);<br/>            let Element &#123; key, value &#125; &#61; vector::pop_back(data);<br/>            return (std::option::some(key), std::option::some(value))<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    vector::push_back(&amp;mut map.data, Element &#123; key, value &#125;);<br/>    (std::option::none(), std::option::none())<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_keys"></a>

## Function `keys`

Return all keys in the map. This requires keys to be copyable.


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt; &#123;<br/>    vector::map_ref(&amp;map.data, &#124;e&#124; &#123;<br/>        let e: &amp;Element&lt;Key, Value&gt; &#61; e;<br/>        e.key<br/>    &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_values"></a>

## Function `values`

Return all values in the map. This requires values to be copyable.


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt; &#123;<br/>    vector::map_ref(&amp;map.data, &#124;e&#124; &#123;<br/>        let e: &amp;Element&lt;Key, Value&gt; &#61; e;<br/>        e.value<br/>    &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_to_vec_pair"></a>

## Function `to_vec_pair`

Transform the map into two vectors with the keys and values respectively
Primarily used to destroy a map


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(<br/>    map: SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;) &#123;<br/>    let keys: vector&lt;Key&gt; &#61; vector::empty();<br/>    let values: vector&lt;Value&gt; &#61; vector::empty();<br/>    let SimpleMap &#123; data &#125; &#61; map;<br/>    vector::for_each(data, &#124;e&#124; &#123;<br/>        let Element &#123; key, value &#125; &#61; e;<br/>        vector::push_back(&amp;mut keys, key);<br/>        vector::push_back(&amp;mut values, value);<br/>    &#125;);<br/>    (keys, values)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_destroy"></a>

## Function `destroy`

For maps that cannot be dropped this is a utility to destroy them
using lambdas to destroy the individual keys and values.


<pre><code>public fun destroy&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;, dk: &#124;Key&#124;, dv: &#124;Value&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun destroy&lt;Key: store, Value: store&gt;(<br/>    map: SimpleMap&lt;Key, Value&gt;,<br/>    dk: &#124;Key&#124;,<br/>    dv: &#124;Value&#124;<br/>) &#123;<br/>    let (keys, values) &#61; to_vec_pair(map);<br/>    vector::destroy(keys, &#124;_k&#124; dk(_k));<br/>    vector::destroy(values, &#124;_v&#124; dv(_v));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_remove"></a>

## Function `remove`

Remove a key/value pair from the map. The key must exist.


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(<br/>    map: &amp;mut SimpleMap&lt;Key, Value&gt;,<br/>    key: &amp;Key,<br/>): (Key, Value) &#123;<br/>    let maybe_idx &#61; find(map, key);<br/>    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));<br/>    let placement &#61; option::extract(&amp;mut maybe_idx);<br/>    let Element &#123; key, value &#125; &#61; vector::swap_remove(&amp;mut map.data, placement);<br/>    (key, value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_simple_map_find"></a>

## Function `find`



<pre><code>fun find&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find&lt;Key: store, Value: store&gt;(<br/>    map: &amp;SimpleMap&lt;Key, Value&gt;,<br/>    key: &amp;Key,<br/>): option::Option&lt;u64&gt; &#123;<br/>    let leng &#61; vector::length(&amp;map.data);<br/>    let i &#61; 0;<br/>    while (i &lt; leng) &#123;<br/>        let element &#61; vector::borrow(&amp;map.data, i);<br/>        if (&amp;element.key &#61;&#61; key) &#123;<br/>            return option::some(i)<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    option::none&lt;u64&gt;()<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SimpleMap"></a>

### Struct `SimpleMap`


<pre><code>struct SimpleMap&lt;Key, Value&gt; has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>data: vector&lt;simple_map::Element&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma intrinsic &#61; map,<br/>    map_new &#61; create,<br/>    map_len &#61; length,<br/>    map_destroy_empty &#61; destroy_empty,<br/>    map_has_key &#61; contains_key,<br/>    map_add_no_override &#61; add,<br/>    map_del_return_key &#61; remove,<br/>    map_borrow &#61; borrow,<br/>    map_borrow_mut &#61; borrow_mut,<br/>    map_spec_get &#61; spec_get,<br/>    map_spec_set &#61; spec_set,<br/>    map_spec_del &#61; spec_remove,<br/>    map_spec_len &#61; spec_len,<br/>    map_spec_has_key &#61; spec_contains_key;<br/></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): u64<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>




<pre><code>pragma intrinsic;<br/>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] spec_len(result) &#61;&#61; 0;<br/>ensures [abstract] forall k: Key: !spec_contains_key(result, k);<br/></code></pre>



<a id="@Specification_1_new_from"></a>

### Function `new_from`


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>




<pre><code>pragma intrinsic;<br/>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] spec_len(result) &#61;&#61; len(keys);<br/>ensures [abstract] forall k: Key: spec_contains_key(result, k) &lt;&#61;&#61;&gt; vector::spec_contains(keys, k);<br/>ensures [abstract] forall i in 0..len(keys):<br/>    spec_get(result, vector::borrow(keys, i)) &#61;&#61; vector::borrow(values, i);<br/></code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code>&#35;[deprecated]<br/>public fun create&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;Value<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;mut Value<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): bool<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value): (option::Option&lt;Key&gt;, option::Option&lt;Value&gt;)<br/></code></pre>




<pre><code>pragma intrinsic;<br/>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] !spec_contains_key(old(map), key) &#61;&#61;&gt; option::is_none(result_1);<br/>ensures [abstract] !spec_contains_key(old(map), key) &#61;&#61;&gt; option::is_none(result_2);<br/>ensures [abstract] spec_contains_key(map, key);<br/>ensures [abstract] spec_get(map, key) &#61;&#61; value;<br/>ensures [abstract] spec_contains_key(old(map), key) &#61;&#61;&gt; ((option::is_some(result_1)) &amp;&amp; (option::spec_borrow(result_1) &#61;&#61; key));<br/>ensures [abstract] spec_contains_key(old(map), key) &#61;&#61;&gt; ((option::is_some(result_2)) &amp;&amp; (option::spec_borrow(result_2) &#61;&#61; spec_get(old(map), key)));<br/></code></pre>




<a id="0x1_simple_map_spec_len"></a>


<pre><code>native fun spec_len&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;): num;<br/></code></pre>




<a id="0x1_simple_map_spec_contains_key"></a>


<pre><code>native fun spec_contains_key&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): bool;<br/></code></pre>




<a id="0x1_simple_map_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K, v: V): SimpleMap&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_simple_map_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): SimpleMap&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_simple_map_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): V;<br/></code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt;<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt;<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>



<a id="@Specification_1_to_vec_pair"></a>

### Function `to_vec_pair`


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;)<br/></code></pre>




<pre><code>pragma intrinsic;<br/>pragma opaque;<br/>ensures [abstract]<br/>    forall k: Key: vector::spec_contains(result_1, k) &lt;&#61;&#61;&gt;<br/>        spec_contains_key(map, k);<br/>ensures [abstract] forall i in 0..len(result_1):<br/>    spec_get(map, vector::borrow(result_1, i)) &#61;&#61; vector::borrow(result_2, i);<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_find"></a>

### Function `find`


<pre><code>fun find&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): option::Option&lt;u64&gt;<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
