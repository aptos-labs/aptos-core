
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


<pre><code>use 0x1::error;
use 0x1::option;
use 0x1::vector;
</code></pre>



<a id="0x1_simple_map_SimpleMap"></a>

## Struct `SimpleMap`



<pre><code>struct SimpleMap&lt;Key, Value&gt; has copy, drop, store
</code></pre>



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



<pre><code>struct Element&lt;Key, Value&gt; has copy, drop, store
</code></pre>



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


<pre><code>const EKEY_ALREADY_EXISTS: u64 &#61; 1;
</code></pre>



<a id="0x1_simple_map_EKEY_NOT_FOUND"></a>

Map key is not found


<pre><code>const EKEY_NOT_FOUND: u64 &#61; 2;
</code></pre>



<a id="0x1_simple_map_length"></a>

## Function `length`



<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): u64 &#123;
    vector::length(&amp;map.data)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_new"></a>

## Function `new`

Create an empty SimpleMap.


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): SimpleMap&lt;Key, Value&gt; &#123;
    SimpleMap &#123;
        data: vector::empty(),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_new_from"></a>

## Function `new_from`

Create a SimpleMap from a vector of keys and values. The keys must be unique.


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(
    keys: vector&lt;Key&gt;,
    values: vector&lt;Value&gt;,
): SimpleMap&lt;Key, Value&gt; &#123;
    let map &#61; new();
    add_all(&amp;mut map, keys, values);
    map
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_create"></a>

## Function `create`

Create an empty SimpleMap.
This function is deprecated, use <code>new</code> instead.


<pre><code>&#35;[deprecated]
public fun create&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create&lt;Key: store, Value: store&gt;(): SimpleMap&lt;Key, Value&gt; &#123;
    new()
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_borrow"></a>

## Function `borrow`



<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(
    map: &amp;SimpleMap&lt;Key, Value&gt;,
    key: &amp;Key,
): &amp;Value &#123;
    let maybe_idx &#61; find(map, key);
    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
    let idx &#61; option::extract(&amp;mut maybe_idx);
    &amp;vector::borrow(&amp;map.data, idx).value
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_borrow_mut"></a>

## Function `borrow_mut`



<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;mut Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(
    map: &amp;mut SimpleMap&lt;Key, Value&gt;,
    key: &amp;Key,
): &amp;mut Value &#123;
    let maybe_idx &#61; find(map, key);
    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
    let idx &#61; option::extract(&amp;mut maybe_idx);
    &amp;mut vector::borrow_mut(&amp;mut map.data, idx).value
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_contains_key"></a>

## Function `contains_key`



<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(
    map: &amp;SimpleMap&lt;Key, Value&gt;,
    key: &amp;Key,
): bool &#123;
    let maybe_idx &#61; find(map, key);
    option::is_some(&amp;maybe_idx)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_destroy_empty"></a>

## Function `destroy_empty`



<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: SimpleMap&lt;Key, Value&gt;) &#123;
    let SimpleMap &#123; data &#125; &#61; map;
    vector::destroy_empty(data);
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_add"></a>

## Function `add`

Add a key/value pair to the map. The key must not already exist.


<pre><code>public fun add&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;Key: store, Value: store&gt;(
    map: &amp;mut SimpleMap&lt;Key, Value&gt;,
    key: Key,
    value: Value,
) &#123;
    let maybe_idx &#61; find(map, &amp;key);
    assert!(option::is_none(&amp;maybe_idx), error::invalid_argument(EKEY_ALREADY_EXISTS));

    vector::push_back(&amp;mut map.data, Element &#123; key, value &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the map. The keys must not already exist.


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(
    map: &amp;mut SimpleMap&lt;Key, Value&gt;,
    keys: vector&lt;Key&gt;,
    values: vector&lt;Value&gt;,
) &#123;
    vector::zip(keys, values, &#124;key, value&#124; &#123;
        add(map, key, value);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_upsert"></a>

## Function `upsert`

Insert key/value pair or update an existing key to a new value


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value): (option::Option&lt;Key&gt;, option::Option&lt;Value&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(
    map: &amp;mut SimpleMap&lt;Key, Value&gt;,
    key: Key,
    value: Value
): (std::option::Option&lt;Key&gt;, std::option::Option&lt;Value&gt;) &#123;
    let data &#61; &amp;mut map.data;
    let len &#61; vector::length(data);
    let i &#61; 0;
    while (i &lt; len) &#123;
        let element &#61; vector::borrow(data, i);
        if (&amp;element.key &#61;&#61; &amp;key) &#123;
            vector::push_back(data, Element &#123; key, value &#125;);
            vector::swap(data, i, len);
            let Element &#123; key, value &#125; &#61; vector::pop_back(data);
            return (std::option::some(key), std::option::some(value))
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    vector::push_back(&amp;mut map.data, Element &#123; key, value &#125;);
    (std::option::none(), std::option::none())
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_keys"></a>

## Function `keys`

Return all keys in the map. This requires keys to be copyable.


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt; &#123;
    vector::map_ref(&amp;map.data, &#124;e&#124; &#123;
        let e: &amp;Element&lt;Key, Value&gt; &#61; e;
        e.key
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_values"></a>

## Function `values`

Return all values in the map. This requires values to be copyable.


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt; &#123;
    vector::map_ref(&amp;map.data, &#124;e&#124; &#123;
        let e: &amp;Element&lt;Key, Value&gt; &#61; e;
        e.value
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_to_vec_pair"></a>

## Function `to_vec_pair`

Transform the map into two vectors with the keys and values respectively
Primarily used to destroy a map


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(
    map: SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;) &#123;
    let keys: vector&lt;Key&gt; &#61; vector::empty();
    let values: vector&lt;Value&gt; &#61; vector::empty();
    let SimpleMap &#123; data &#125; &#61; map;
    vector::for_each(data, &#124;e&#124; &#123;
        let Element &#123; key, value &#125; &#61; e;
        vector::push_back(&amp;mut keys, key);
        vector::push_back(&amp;mut values, value);
    &#125;);
    (keys, values)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_destroy"></a>

## Function `destroy`

For maps that cannot be dropped this is a utility to destroy them
using lambdas to destroy the individual keys and values.


<pre><code>public fun destroy&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;, dk: &#124;Key&#124;, dv: &#124;Value&#124;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun destroy&lt;Key: store, Value: store&gt;(
    map: SimpleMap&lt;Key, Value&gt;,
    dk: &#124;Key&#124;,
    dv: &#124;Value&#124;
) &#123;
    let (keys, values) &#61; to_vec_pair(map);
    vector::destroy(keys, &#124;_k&#124; dk(_k));
    vector::destroy(values, &#124;_v&#124; dv(_v));
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_remove"></a>

## Function `remove`

Remove a key/value pair from the map. The key must exist.


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(
    map: &amp;mut SimpleMap&lt;Key, Value&gt;,
    key: &amp;Key,
): (Key, Value) &#123;
    let maybe_idx &#61; find(map, key);
    assert!(option::is_some(&amp;maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
    let placement &#61; option::extract(&amp;mut maybe_idx);
    let Element &#123; key, value &#125; &#61; vector::swap_remove(&amp;mut map.data, placement);
    (key, value)
&#125;
</code></pre>



</details>

<a id="0x1_simple_map_find"></a>

## Function `find`



<pre><code>fun find&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find&lt;Key: store, Value: store&gt;(
    map: &amp;SimpleMap&lt;Key, Value&gt;,
    key: &amp;Key,
): option::Option&lt;u64&gt; &#123;
    let leng &#61; vector::length(&amp;map.data);
    let i &#61; 0;
    while (i &lt; leng) &#123;
        let element &#61; vector::borrow(&amp;map.data, i);
        if (&amp;element.key &#61;&#61; key) &#123;
            return option::some(i)
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    option::none&lt;u64&gt;()
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SimpleMap"></a>

### Struct `SimpleMap`


<pre><code>struct SimpleMap&lt;Key, Value&gt; has copy, drop, store
</code></pre>



<dl>
<dt>
<code>data: vector&lt;simple_map::Element&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma intrinsic &#61; map,
    map_new &#61; create,
    map_len &#61; length,
    map_destroy_empty &#61; destroy_empty,
    map_has_key &#61; contains_key,
    map_add_no_override &#61; add,
    map_del_return_key &#61; remove,
    map_borrow &#61; borrow,
    map_borrow_mut &#61; borrow_mut,
    map_spec_get &#61; spec_get,
    map_spec_set &#61; spec_set,
    map_spec_del &#61; spec_remove,
    map_spec_len &#61; spec_len,
    map_spec_has_key &#61; spec_contains_key;
</code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code>public fun length&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): u64
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public fun new&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>




<pre><code>pragma intrinsic;
pragma opaque;
aborts_if [abstract] false;
ensures [abstract] spec_len(result) &#61;&#61; 0;
ensures [abstract] forall k: Key: !spec_contains_key(result, k);
</code></pre>



<a id="@Specification_1_new_from"></a>

### Function `new_from`


<pre><code>public fun new_from&lt;Key: store, Value: store&gt;(keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>




<pre><code>pragma intrinsic;
pragma opaque;
aborts_if [abstract] false;
ensures [abstract] spec_len(result) &#61;&#61; len(keys);
ensures [abstract] forall k: Key: spec_contains_key(result, k) &lt;&#61;&#61;&gt; vector::spec_contains(keys, k);
ensures [abstract] forall i in 0..len(keys):
    spec_get(result, vector::borrow(keys, i)) &#61;&#61; vector::borrow(values, i);
</code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code>&#35;[deprecated]
public fun create&lt;Key: store, Value: store&gt;(): simple_map::SimpleMap&lt;Key, Value&gt;
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;Value
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): &amp;mut Value
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code>public fun contains_key&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): bool
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code>public fun add_all&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, keys: vector&lt;Key&gt;, values: vector&lt;Value&gt;)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code>public fun upsert&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: Key, value: Value): (option::Option&lt;Key&gt;, option::Option&lt;Value&gt;)
</code></pre>




<pre><code>pragma intrinsic;
pragma opaque;
aborts_if [abstract] false;
ensures [abstract] !spec_contains_key(old(map), key) &#61;&#61;&gt; option::is_none(result_1);
ensures [abstract] !spec_contains_key(old(map), key) &#61;&#61;&gt; option::is_none(result_2);
ensures [abstract] spec_contains_key(map, key);
ensures [abstract] spec_get(map, key) &#61;&#61; value;
ensures [abstract] spec_contains_key(old(map), key) &#61;&#61;&gt; ((option::is_some(result_1)) &amp;&amp; (option::spec_borrow(result_1) &#61;&#61; key));
ensures [abstract] spec_contains_key(old(map), key) &#61;&#61;&gt; ((option::is_some(result_2)) &amp;&amp; (option::spec_borrow(result_2) &#61;&#61; spec_get(old(map), key)));
</code></pre>




<a id="0x1_simple_map_spec_len"></a>


<pre><code>native fun spec_len&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;): num;
</code></pre>




<a id="0x1_simple_map_spec_contains_key"></a>


<pre><code>native fun spec_contains_key&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_simple_map_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K, v: V): SimpleMap&lt;K, V&gt;;
</code></pre>




<a id="0x1_simple_map_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): SimpleMap&lt;K, V&gt;;
</code></pre>




<a id="0x1_simple_map_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: SimpleMap&lt;K, V&gt;, k: K): V;
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code>public fun keys&lt;Key: copy, Value&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Key&gt;
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code>public fun values&lt;Key, Value: copy&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;): vector&lt;Value&gt;
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>



<a id="@Specification_1_to_vec_pair"></a>

### Function `to_vec_pair`


<pre><code>public fun to_vec_pair&lt;Key: store, Value: store&gt;(map: simple_map::SimpleMap&lt;Key, Value&gt;): (vector&lt;Key&gt;, vector&lt;Value&gt;)
</code></pre>




<pre><code>pragma intrinsic;
pragma opaque;
ensures [abstract]
    forall k: Key: vector::spec_contains(result_1, k) &lt;&#61;&#61;&gt;
        spec_contains_key(map, k);
ensures [abstract] forall i in 0..len(result_1):
    spec_get(map, vector::borrow(result_1, i)) &#61;&#61; vector::borrow(result_2, i);
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;Key: store, Value: store&gt;(map: &amp;mut simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): (Key, Value)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_1_find"></a>

### Function `find`


<pre><code>fun find&lt;Key: store, Value: store&gt;(map: &amp;simple_map::SimpleMap&lt;Key, Value&gt;, key: &amp;Key): option::Option&lt;u64&gt;
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
