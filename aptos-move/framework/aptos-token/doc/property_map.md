
<a id="0x3_property_map"></a>

# Module `0x3::property_map`

PropertyMap is a specialization of SimpleMap for Tokens.
It maps a String key to a PropertyValue that consists of type (string) and value (vector<u8>)
It provides basic on-chain serialization of primitive and string to property value with type information
It also supports deserializing property value to it original type.


-  [Struct `PropertyMap`](#0x3_property_map_PropertyMap)
-  [Struct `PropertyValue`](#0x3_property_map_PropertyValue)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x3_property_map_new)
-  [Function `new_with_key_and_property_value`](#0x3_property_map_new_with_key_and_property_value)
-  [Function `empty`](#0x3_property_map_empty)
-  [Function `contains_key`](#0x3_property_map_contains_key)
-  [Function `add`](#0x3_property_map_add)
-  [Function `length`](#0x3_property_map_length)
-  [Function `borrow`](#0x3_property_map_borrow)
-  [Function `keys`](#0x3_property_map_keys)
-  [Function `types`](#0x3_property_map_types)
-  [Function `values`](#0x3_property_map_values)
-  [Function `read_string`](#0x3_property_map_read_string)
-  [Function `read_u8`](#0x3_property_map_read_u8)
-  [Function `read_u64`](#0x3_property_map_read_u64)
-  [Function `read_address`](#0x3_property_map_read_address)
-  [Function `read_u128`](#0x3_property_map_read_u128)
-  [Function `read_bool`](#0x3_property_map_read_bool)
-  [Function `borrow_value`](#0x3_property_map_borrow_value)
-  [Function `borrow_type`](#0x3_property_map_borrow_type)
-  [Function `remove`](#0x3_property_map_remove)
-  [Function `update_property_map`](#0x3_property_map_update_property_map)
-  [Function `update_property_value`](#0x3_property_map_update_property_value)
-  [Function `create_property_value_raw`](#0x3_property_map_create_property_value_raw)
-  [Function `create_property_value`](#0x3_property_map_create_property_value)
-  [Specification](#@Specification_1)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `new_with_key_and_property_value`](#@Specification_1_new_with_key_and_property_value)
    -  [Function `empty`](#@Specification_1_empty)
    -  [Function `contains_key`](#@Specification_1_contains_key)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `length`](#@Specification_1_length)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `keys`](#@Specification_1_keys)
    -  [Function `types`](#@Specification_1_types)
    -  [Function `values`](#@Specification_1_values)
    -  [Function `read_string`](#@Specification_1_read_string)
    -  [Function `read_u8`](#@Specification_1_read_u8)
    -  [Function `read_u64`](#@Specification_1_read_u64)
    -  [Function `read_address`](#@Specification_1_read_address)
    -  [Function `read_u128`](#@Specification_1_read_u128)
    -  [Function `read_bool`](#@Specification_1_read_bool)
    -  [Function `borrow_value`](#@Specification_1_borrow_value)
    -  [Function `borrow_type`](#@Specification_1_borrow_type)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `update_property_map`](#@Specification_1_update_property_map)
    -  [Function `update_property_value`](#@Specification_1_update_property_value)
    -  [Function `create_property_value_raw`](#@Specification_1_create_property_value_raw)
    -  [Function `create_property_value`](#@Specification_1_create_property_value)


<pre><code>use 0x1::bcs;
use 0x1::error;
use 0x1::from_bcs;
use 0x1::simple_map;
use 0x1::string;
use 0x1::type_info;
</code></pre>



<a id="0x3_property_map_PropertyMap"></a>

## Struct `PropertyMap`



<pre><code>struct PropertyMap has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>map: simple_map::SimpleMap&lt;string::String, property_map::PropertyValue&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_property_map_PropertyValue"></a>

## Struct `PropertyValue`



<pre><code>struct PropertyValue has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>type: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code>const EKEY_AREADY_EXIST_IN_PROPERTY_MAP: u64 &#61; 1;
</code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT"></a>

Property key and type count don't match


<pre><code>const EKEY_COUNT_NOT_MATCH_TYPE_COUNT: u64 &#61; 5;
</code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT"></a>

Property key and value count don't match


<pre><code>const EKEY_COUNT_NOT_MATCH_VALUE_COUNT: u64 &#61; 4;
</code></pre>



<a id="0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG"></a>

The name (key) of the property is too long


<pre><code>const EPROPERTY_MAP_NAME_TOO_LONG: u64 &#61; 7;
</code></pre>



<a id="0x3_property_map_EPROPERTY_NOT_EXIST"></a>

The property doesn't exist


<pre><code>const EPROPERTY_NOT_EXIST: u64 &#61; 3;
</code></pre>



<a id="0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT"></a>

The number of property exceeds the limit


<pre><code>const EPROPERTY_NUMBER_EXCEED_LIMIT: u64 &#61; 2;
</code></pre>



<a id="0x3_property_map_ETYPE_NOT_MATCH"></a>

Property type doesn't match


<pre><code>const ETYPE_NOT_MATCH: u64 &#61; 6;
</code></pre>



<a id="0x3_property_map_MAX_PROPERTY_MAP_SIZE"></a>

The maximal number of property that can be stored in property map


<pre><code>const MAX_PROPERTY_MAP_SIZE: u64 &#61; 1000;
</code></pre>



<a id="0x3_property_map_MAX_PROPERTY_NAME_LENGTH"></a>



<pre><code>const MAX_PROPERTY_NAME_LENGTH: u64 &#61; 128;
</code></pre>



<a id="0x3_property_map_new"></a>

## Function `new`



<pre><code>public fun new(keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): property_map::PropertyMap
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new(
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;
): PropertyMap &#123;
    let length &#61; vector::length(&amp;keys);
    assert!(length &lt;&#61; MAX_PROPERTY_MAP_SIZE, error::invalid_argument(EPROPERTY_NUMBER_EXCEED_LIMIT));
    assert!(length &#61;&#61; vector::length(&amp;values), error::invalid_argument(EKEY_COUNT_NOT_MATCH_VALUE_COUNT));
    assert!(length &#61;&#61; vector::length(&amp;types), error::invalid_argument(EKEY_COUNT_NOT_MATCH_TYPE_COUNT));

    let properties &#61; empty();

    let i &#61; 0;
    while (i &lt; length) &#123;
        let key &#61; &#42;vector::borrow(&amp;keys, i);
        assert!(string::length(&amp;key) &lt;&#61; MAX_PROPERTY_NAME_LENGTH, error::invalid_argument(EPROPERTY_MAP_NAME_TOO_LONG));
        simple_map::add(
            &amp;mut properties.map,
            key,
            PropertyValue &#123; value: &#42;vector::borrow(&amp;values, i), type: &#42;vector::borrow(&amp;types, i) &#125;
        );
        i &#61; i &#43; 1;
    &#125;;
    properties
&#125;
</code></pre>



</details>

<a id="0x3_property_map_new_with_key_and_property_value"></a>

## Function `new_with_key_and_property_value`

Create property map directly from key and property value


<pre><code>public fun new_with_key_and_property_value(keys: vector&lt;string::String&gt;, values: vector&lt;property_map::PropertyValue&gt;): property_map::PropertyMap
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_with_key_and_property_value(
    keys: vector&lt;String&gt;,
    values: vector&lt;PropertyValue&gt;
): PropertyMap &#123;
    let length &#61; vector::length(&amp;keys);
    assert!(length &lt;&#61; MAX_PROPERTY_MAP_SIZE, error::invalid_argument(EPROPERTY_NUMBER_EXCEED_LIMIT));
    assert!(length &#61;&#61; vector::length(&amp;values), error::invalid_argument(EKEY_COUNT_NOT_MATCH_VALUE_COUNT));

    let properties &#61; empty();

    let i &#61; 0;
    while (i &lt; length) &#123;
        let key &#61; &#42;vector::borrow(&amp;keys, i);
        let val &#61; &#42;vector::borrow(&amp;values, i);
        assert!(string::length(&amp;key) &lt;&#61; MAX_PROPERTY_NAME_LENGTH, error::invalid_argument(EPROPERTY_MAP_NAME_TOO_LONG));
        add(&amp;mut properties, key, val);
        i &#61; i &#43; 1;
    &#125;;
    properties
&#125;
</code></pre>



</details>

<a id="0x3_property_map_empty"></a>

## Function `empty`



<pre><code>public fun empty(): property_map::PropertyMap
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun empty(): PropertyMap &#123;
    PropertyMap &#123;
        map: simple_map::create&lt;String, PropertyValue&gt;(),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_property_map_contains_key"></a>

## Function `contains_key`



<pre><code>public fun contains_key(map: &amp;property_map::PropertyMap, key: &amp;string::String): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains_key(map: &amp;PropertyMap, key: &amp;String): bool &#123;
    simple_map::contains_key(&amp;map.map, key)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_add"></a>

## Function `add`



<pre><code>public fun add(map: &amp;mut property_map::PropertyMap, key: string::String, value: property_map::PropertyValue)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(map: &amp;mut PropertyMap, key: String, value: PropertyValue) &#123;
    assert!(string::length(&amp;key) &lt;&#61; MAX_PROPERTY_NAME_LENGTH, error::invalid_argument(EPROPERTY_MAP_NAME_TOO_LONG));
    assert!(simple_map::length(&amp;map.map) &lt; MAX_PROPERTY_MAP_SIZE, error::invalid_state(EPROPERTY_NUMBER_EXCEED_LIMIT));
    simple_map::add(&amp;mut map.map, key, value);
&#125;
</code></pre>



</details>

<a id="0x3_property_map_length"></a>

## Function `length`



<pre><code>public fun length(map: &amp;property_map::PropertyMap): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length(map: &amp;PropertyMap): u64 &#123;
    simple_map::length(&amp;map.map)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_borrow"></a>

## Function `borrow`



<pre><code>public fun borrow(map: &amp;property_map::PropertyMap, key: &amp;string::String): &amp;property_map::PropertyValue
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow(map: &amp;PropertyMap, key: &amp;String): &amp;PropertyValue &#123;
    let found &#61; contains_key(map, key);
    assert!(found, EPROPERTY_NOT_EXIST);
    simple_map::borrow(&amp;map.map, key)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_keys"></a>

## Function `keys`

Return all the keys in the property map in the order they are added.


<pre><code>public fun keys(map: &amp;property_map::PropertyMap): vector&lt;string::String&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys(map: &amp;PropertyMap): vector&lt;String&gt; &#123;
    simple_map::keys(&amp;map.map)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_types"></a>

## Function `types`

Return the types of all properties in the property map in the order they are added.


<pre><code>public fun types(map: &amp;property_map::PropertyMap): vector&lt;string::String&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun types(map: &amp;PropertyMap): vector&lt;String&gt; &#123;
    vector::map_ref(&amp;simple_map::values(&amp;map.map), &#124;v&#124; &#123;
        let v: &amp;PropertyValue &#61; v;
        v.type
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_values"></a>

## Function `values`

Return the values of all properties in the property map in the order they are added.


<pre><code>public fun values(map: &amp;property_map::PropertyMap): vector&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun values(map: &amp;PropertyMap): vector&lt;vector&lt;u8&gt;&gt; &#123;
    vector::map_ref(&amp;simple_map::values(&amp;map.map), &#124;v&#124; &#123;
        let v: &amp;PropertyValue &#61; v;
        v.value
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_string"></a>

## Function `read_string`



<pre><code>public fun read_string(map: &amp;property_map::PropertyMap, key: &amp;string::String): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_string(map: &amp;PropertyMap, key: &amp;String): String &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;0x1::string::String&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_string(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_u8"></a>

## Function `read_u8`



<pre><code>public fun read_u8(map: &amp;property_map::PropertyMap, key: &amp;string::String): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u8(map: &amp;PropertyMap, key: &amp;String): u8 &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;u8&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_u8(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_u64"></a>

## Function `read_u64`



<pre><code>public fun read_u64(map: &amp;property_map::PropertyMap, key: &amp;string::String): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u64(map: &amp;PropertyMap, key: &amp;String): u64 &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;u64&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_u64(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_address"></a>

## Function `read_address`



<pre><code>public fun read_address(map: &amp;property_map::PropertyMap, key: &amp;string::String): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_address(map: &amp;PropertyMap, key: &amp;String): address &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;address&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_address(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_u128"></a>

## Function `read_u128`



<pre><code>public fun read_u128(map: &amp;property_map::PropertyMap, key: &amp;string::String): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u128(map: &amp;PropertyMap, key: &amp;String): u128 &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;u128&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_u128(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_read_bool"></a>

## Function `read_bool`



<pre><code>public fun read_bool(map: &amp;property_map::PropertyMap, key: &amp;string::String): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_bool(map: &amp;PropertyMap, key: &amp;String): bool &#123;
    let prop &#61; borrow(map, key);
    assert!(prop.type &#61;&#61; string::utf8(b&quot;bool&quot;), error::invalid_state(ETYPE_NOT_MATCH));
    from_bcs::to_bool(prop.value)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_borrow_value"></a>

## Function `borrow_value`



<pre><code>public fun borrow_value(property: &amp;property_map::PropertyValue): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_value(property: &amp;PropertyValue): vector&lt;u8&gt; &#123;
    property.value
&#125;
</code></pre>



</details>

<a id="0x3_property_map_borrow_type"></a>

## Function `borrow_type`



<pre><code>public fun borrow_type(property: &amp;property_map::PropertyValue): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_type(property: &amp;PropertyValue): String &#123;
    property.type
&#125;
</code></pre>



</details>

<a id="0x3_property_map_remove"></a>

## Function `remove`



<pre><code>public fun remove(map: &amp;mut property_map::PropertyMap, key: &amp;string::String): (string::String, property_map::PropertyValue)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove(
    map: &amp;mut PropertyMap,
    key: &amp;String
): (String, PropertyValue) &#123;
    let found &#61; contains_key(map, key);
    assert!(found, error::not_found(EPROPERTY_NOT_EXIST));
    simple_map::remove(&amp;mut map.map, key)
&#125;
</code></pre>



</details>

<a id="0x3_property_map_update_property_map"></a>

## Function `update_property_map`

Update the property in the existing property map
Allow updating existing keys' value and add new key-value pairs


<pre><code>public fun update_property_map(map: &amp;mut property_map::PropertyMap, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_property_map(
    map: &amp;mut PropertyMap,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;,
) &#123;
    let key_len &#61; vector::length(&amp;keys);
    let val_len &#61; vector::length(&amp;values);
    let typ_len &#61; vector::length(&amp;types);
    assert!(key_len &#61;&#61; val_len, error::invalid_state(EKEY_COUNT_NOT_MATCH_VALUE_COUNT));
    assert!(key_len &#61;&#61; typ_len, error::invalid_state(EKEY_COUNT_NOT_MATCH_TYPE_COUNT));

    let i &#61; 0;
    while (i &lt; key_len) &#123;
        let key &#61; vector::borrow(&amp;keys, i);
        let prop_val &#61; PropertyValue &#123;
            value: &#42;vector::borrow(&amp;values, i),
            type: &#42;vector::borrow(&amp;types, i),
        &#125;;
        if (contains_key(map, key)) &#123;
            update_property_value(map, key, prop_val);
        &#125; else &#123;
            add(map, &#42;key, prop_val);
        &#125;;
        i &#61; i &#43; 1;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_property_map_update_property_value"></a>

## Function `update_property_value`



<pre><code>public fun update_property_value(map: &amp;mut property_map::PropertyMap, key: &amp;string::String, value: property_map::PropertyValue)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_property_value(
    map: &amp;mut PropertyMap,
    key: &amp;String,
    value: PropertyValue
) &#123;
    let property_val &#61; simple_map::borrow_mut(&amp;mut map.map, key);
    &#42;property_val &#61; value;
&#125;
</code></pre>



</details>

<a id="0x3_property_map_create_property_value_raw"></a>

## Function `create_property_value_raw`



<pre><code>public fun create_property_value_raw(value: vector&lt;u8&gt;, type: string::String): property_map::PropertyValue
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_property_value_raw(
    value: vector&lt;u8&gt;,
    type: String
): PropertyValue &#123;
    PropertyValue &#123;
        value,
        type,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_property_map_create_property_value"></a>

## Function `create_property_value`

create a property value from generic type data


<pre><code>public fun create_property_value&lt;T: copy&gt;(data: &amp;T): property_map::PropertyValue
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_property_value&lt;T: copy&gt;(data: &amp;T): PropertyValue &#123;
    let name &#61; type_name&lt;T&gt;();
    if (
        name &#61;&#61; string::utf8(b&quot;bool&quot;) &#124;&#124;
            name &#61;&#61; string::utf8(b&quot;u8&quot;) &#124;&#124;
            name &#61;&#61; string::utf8(b&quot;u64&quot;) &#124;&#124;
            name &#61;&#61; string::utf8(b&quot;u128&quot;) &#124;&#124;
            name &#61;&#61; string::utf8(b&quot;address&quot;) &#124;&#124;
            name &#61;&#61; string::utf8(b&quot;0x1::string::String&quot;)
    ) &#123;
        create_property_value_raw(bcs::to_bytes&lt;T&gt;(data), name)
    &#125; else &#123;
        create_property_value_raw(bcs::to_bytes&lt;T&gt;(data), string::utf8(b&quot;vector&lt;u8&gt;&quot;))
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
let MAX_PROPERTY_MAP_SIZE &#61; 1000;
let MAX_PROPERTY_NAME_LENGTH  &#61; 128;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public fun new(keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): property_map::PropertyMap
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let length &#61; len(keys);
aborts_if !(length &lt;&#61; MAX_PROPERTY_MAP_SIZE);
aborts_if !(length &#61;&#61; vector::length(values));
aborts_if !(length &#61;&#61; vector::length(types));
</code></pre>



<a id="@Specification_1_new_with_key_and_property_value"></a>

### Function `new_with_key_and_property_value`


<pre><code>public fun new_with_key_and_property_value(keys: vector&lt;string::String&gt;, values: vector&lt;property_map::PropertyValue&gt;): property_map::PropertyMap
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let length &#61; vector::length(keys);
aborts_if !(length &lt;&#61; MAX_PROPERTY_MAP_SIZE);
aborts_if !(length &#61;&#61; len(values));
</code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>public fun empty(): property_map::PropertyMap
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code>public fun contains_key(map: &amp;property_map::PropertyMap, key: &amp;string::String): bool
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(map: &amp;mut property_map::PropertyMap, key: string::String, value: property_map::PropertyValue)
</code></pre>




<pre><code>aborts_if !(string::length(key) &lt;&#61; MAX_PROPERTY_NAME_LENGTH);
aborts_if !(!simple_map::spec_contains_key(map.map, key));
aborts_if !(simple_map::spec_len(map.map) &lt; MAX_PROPERTY_MAP_SIZE);
</code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code>public fun length(map: &amp;property_map::PropertyMap): u64
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow(map: &amp;property_map::PropertyMap, key: &amp;string::String): &amp;property_map::PropertyValue
</code></pre>




<pre><code>aborts_if !simple_map::spec_contains_key(map.map, key);
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code>public fun keys(map: &amp;property_map::PropertyMap): vector&lt;string::String&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_types"></a>

### Function `types`


<pre><code>public fun types(map: &amp;property_map::PropertyMap): vector&lt;string::String&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code>public fun values(map: &amp;property_map::PropertyMap): vector&lt;vector&lt;u8&gt;&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_read_string"></a>

### Function `read_string`


<pre><code>public fun read_string(map: &amp;property_map::PropertyMap, key: &amp;string::String): string::String
</code></pre>


Check utf8 for correctness and whether equal
to <code>prop.type</code>


<pre><code>pragma aborts_if_is_partial;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(b&quot;0x1::string::String&quot;);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(b&quot;0x1::string::String&quot;);
aborts_if !aptos_std::from_bcs::deserializable&lt;String&gt;(prop.value);
</code></pre>




<a id="0x3_property_map_spec_utf8"></a>


<pre><code>fun spec_utf8(bytes: vector&lt;u8&gt;): String &#123;
   String&#123;bytes&#125;
&#125;
</code></pre>



<a id="@Specification_1_read_u8"></a>

### Function `read_u8`


<pre><code>public fun read_u8(map: &amp;property_map::PropertyMap, key: &amp;string::String): u8
</code></pre>




<pre><code>let str &#61; b&quot;u8&quot;;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(str);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(str);
aborts_if !aptos_std::from_bcs::deserializable&lt;u8&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_u64"></a>

### Function `read_u64`


<pre><code>public fun read_u64(map: &amp;property_map::PropertyMap, key: &amp;string::String): u64
</code></pre>




<pre><code>let str &#61; b&quot;u64&quot;;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(str);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(str);
aborts_if !aptos_std::from_bcs::deserializable&lt;u64&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_address"></a>

### Function `read_address`


<pre><code>public fun read_address(map: &amp;property_map::PropertyMap, key: &amp;string::String): address
</code></pre>




<pre><code>let str &#61; b&quot;address&quot;;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(str);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(str);
aborts_if !aptos_std::from_bcs::deserializable&lt;address&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_u128"></a>

### Function `read_u128`


<pre><code>public fun read_u128(map: &amp;property_map::PropertyMap, key: &amp;string::String): u128
</code></pre>




<pre><code>let str &#61; b&quot;u128&quot;;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(str);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(str);
aborts_if !aptos_std::from_bcs::deserializable&lt;u128&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_bool"></a>

### Function `read_bool`


<pre><code>public fun read_bool(map: &amp;property_map::PropertyMap, key: &amp;string::String): bool
</code></pre>




<pre><code>let str &#61; b&quot;bool&quot;;
aborts_if !simple_map::spec_contains_key(map.map, key);
aborts_if !string::spec_internal_check_utf8(str);
let prop &#61; simple_map::spec_get(map.map, key);
aborts_if prop.type !&#61; spec_utf8(str);
aborts_if !aptos_std::from_bcs::deserializable&lt;bool&gt;(prop.value);
</code></pre>



<a id="@Specification_1_borrow_value"></a>

### Function `borrow_value`


<pre><code>public fun borrow_value(property: &amp;property_map::PropertyValue): vector&lt;u8&gt;
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_borrow_type"></a>

### Function `borrow_type`


<pre><code>public fun borrow_type(property: &amp;property_map::PropertyValue): string::String
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove(map: &amp;mut property_map::PropertyMap, key: &amp;string::String): (string::String, property_map::PropertyValue)
</code></pre>




<pre><code>aborts_if !simple_map::spec_contains_key(map.map, key);
</code></pre>



<a id="@Specification_1_update_property_map"></a>

### Function `update_property_map`


<pre><code>public fun update_property_map(map: &amp;mut property_map::PropertyMap, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let key_len &#61; len(keys);
let val_len &#61; len(values);
let typ_len &#61; len(types);
aborts_if !(key_len &#61;&#61; val_len);
aborts_if !(key_len &#61;&#61; typ_len);
</code></pre>



<a id="@Specification_1_update_property_value"></a>

### Function `update_property_value`


<pre><code>public fun update_property_value(map: &amp;mut property_map::PropertyMap, key: &amp;string::String, value: property_map::PropertyValue)
</code></pre>




<pre><code>aborts_if !simple_map::spec_contains_key(map.map, key);
</code></pre>



<a id="@Specification_1_create_property_value_raw"></a>

### Function `create_property_value_raw`


<pre><code>public fun create_property_value_raw(value: vector&lt;u8&gt;, type: string::String): property_map::PropertyValue
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_create_property_value"></a>

### Function `create_property_value`


<pre><code>public fun create_property_value&lt;T: copy&gt;(data: &amp;T): property_map::PropertyValue
</code></pre>


Abort according to the code


<pre><code>let name &#61; type_name&lt;T&gt;();
aborts_if !string::spec_internal_check_utf8(b&quot;bool&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;u8&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u8&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;u64&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u8&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u64&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;u128&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u8&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u64&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u128&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;address&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u8&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u64&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u128&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;address&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;0x1::string::String&quot;);
aborts_if name !&#61; spec_utf8(b&quot;bool&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u8&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u64&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;u128&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;address&quot;) &amp;&amp;
    name !&#61; spec_utf8(b&quot;0x1::string::String&quot;) &amp;&amp;
    !string::spec_internal_check_utf8(b&quot;vector&lt;u8&gt;&quot;);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
