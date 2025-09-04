
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


<pre><code><b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a id="0x3_property_map_PropertyMap"></a>

## Struct `PropertyMap`



<pre><code><b>struct</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>map: <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_property_map_PropertyValue"></a>

## Struct `PropertyValue`



<pre><code><b>struct</b> <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP">EKEY_AREADY_EXIST_IN_PROPERTY_MAP</a>: u64 = 1;
</code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT"></a>

Property key and type count don't match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>: u64 = 5;
</code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT"></a>

Property key and value count don't match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>: u64 = 4;
</code></pre>



<a id="0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG"></a>

The name (key) of the property is too long


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>: u64 = 7;
</code></pre>



<a id="0x3_property_map_EPROPERTY_NOT_EXIST"></a>

The property doesn't exist


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>: u64 = 3;
</code></pre>



<a id="0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT"></a>

The number of property exceeds the limit


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>: u64 = 2;
</code></pre>



<a id="0x3_property_map_ETYPE_NOT_MATCH"></a>

Property type doesn't match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>: u64 = 6;
</code></pre>



<a id="0x3_property_map_MAX_PROPERTY_MAP_SIZE"></a>

The maximal number of property that can be stored in property map


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>: u64 = 1000;
</code></pre>



<a id="0x3_property_map_MAX_PROPERTY_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a id="0x3_property_map_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(
    keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;
): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> length = keys.<a href="property_map.md#0x3_property_map_length">length</a>();
    <b>assert</b>!(<a href="property_map.md#0x3_property_map_length">length</a> &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));
    <b>assert</b>!(length == values.<a href="property_map.md#0x3_property_map_length">length</a>(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));
    <b>assert</b>!(length == types.<a href="property_map.md#0x3_property_map_length">length</a>(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>));

    <b>let</b> properties = <a href="property_map.md#0x3_property_map_empty">empty</a>();

    for (i in 0..length) {
        <b>let</b> key = keys[i];
        <b>assert</b>!(key.<a href="property_map.md#0x3_property_map_length">length</a>() &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));
        properties.map.<a href="property_map.md#0x3_property_map_add">add</a>(
            key,
            <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> { value: values[i], type: types[i] }
        );
    };
    properties
}
</code></pre>



</details>

<a id="0x3_property_map_new_with_key_and_property_value"></a>

## Function `new_with_key_and_property_value`

Create property map directly from key and property value


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(
    keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>&gt;
): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> length = keys.<a href="property_map.md#0x3_property_map_length">length</a>();
    <b>assert</b>!(<a href="property_map.md#0x3_property_map_length">length</a> &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));
    <b>assert</b>!(length == values.<a href="property_map.md#0x3_property_map_length">length</a>(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));

    <b>let</b> properties = <a href="property_map.md#0x3_property_map_empty">empty</a>();

    for (i in 0..length) {
        <b>let</b> key = keys[i];
        <b>let</b> val = values[i];
        <b>assert</b>!(key.<a href="property_map.md#0x3_property_map_length">length</a>() &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));
        properties.<a href="property_map.md#0x3_property_map_add">add</a>(key, val);
    };
    properties
}
</code></pre>



</details>

<a id="0x3_property_map_empty"></a>

## Function `empty`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> {
    <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> {
        map: <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>&gt;(),
    }
}
</code></pre>



</details>

<a id="0x3_property_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): bool {
    self.map.<a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(key)
}
</code></pre>



</details>

<a id="0x3_property_map_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: String, value: <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>) {
    <b>assert</b>!(key.<a href="property_map.md#0x3_property_map_length">length</a>() &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));
    <b>assert</b>!(self.map.<a href="property_map.md#0x3_property_map_length">length</a>() &lt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));
    self.map.<a href="property_map.md#0x3_property_map_add">add</a>(key, value);
}
</code></pre>



</details>

<a id="0x3_property_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): u64 {
    self.map.<a href="property_map.md#0x3_property_map_length">length</a>()
}
</code></pre>



</details>

<a id="0x3_property_map_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): &<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> {
    <b>let</b> found = self.<a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(key);
    <b>assert</b>!(found, <a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>);
    self.map.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key)
}
</code></pre>



</details>

<a id="0x3_property_map_keys"></a>

## Function `keys`

Return all the keys in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; {
    self.map.<a href="property_map.md#0x3_property_map_keys">keys</a>()
}
</code></pre>



</details>

<a id="0x3_property_map_types"></a>

## Function `types`

Return the types of all properties in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; {
    self.map.<a href="property_map.md#0x3_property_map_values">values</a>().map_ref(|v| {
        v.type
    })
}
</code></pre>



</details>

<a id="0x3_property_map_values"></a>

## Function `values`

Return the values of all properties in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    self.map.<a href="property_map.md#0x3_property_map_values">values</a>().map_ref(|v| {
        v.value
    })
}
</code></pre>



</details>

<a id="0x3_property_map_read_string"></a>

## Function `read_string`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): String {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_read_u8"></a>

## Function `read_u8`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): u8 {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u8"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_read_u64"></a>

## Function `read_u64`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): u64 {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u64"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_read_address"></a>

## Function `read_address`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): <b>address</b> {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<b>address</b>"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_read_u128"></a>

## Function `read_u128`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): u128 {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u128"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_read_bool"></a>

## Function `read_bool`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &String): bool {
    <b>let</b> prop = self.<a href="property_map.md#0x3_property_map_borrow">borrow</a>(key);
    <b>assert</b>!(prop.type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"bool"), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(prop.value)
}
</code></pre>



</details>

<a id="0x3_property_map_borrow_value"></a>

## Function `borrow_value`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    self.value
}
</code></pre>



</details>

<a id="0x3_property_map_borrow_type"></a>

## Function `borrow_type`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>): String {
    self.type
}
</code></pre>



</details>

<a id="0x3_property_map_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(
    self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,
    key: &String
): (String, <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>) {
    <b>let</b> found = self.<a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(key);
    <b>assert</b>!(found, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>));
    self.map.<a href="property_map.md#0x3_property_map_remove">remove</a>(key)
}
</code></pre>



</details>

<a id="0x3_property_map_update_property_map"></a>

## Function `update_property_map`

Update the property in the existing property map
Allow updating existing keys' value and add new key-value pairs


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(
    self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,
    keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
) {
    <b>let</b> key_len = keys.<a href="property_map.md#0x3_property_map_length">length</a>();
    <b>let</b> val_len = values.<a href="property_map.md#0x3_property_map_length">length</a>();
    <b>let</b> typ_len = types.<a href="property_map.md#0x3_property_map_length">length</a>();
    <b>assert</b>!(key_len == val_len, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));
    <b>assert</b>!(key_len == typ_len, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>));

    for (i in 0..key_len) {
        <b>let</b> key = &keys[i];
        <b>let</b> prop_val = <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> {
            value: values[i],
            type: types[i],
        };
        <b>if</b> (self.<a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(key)) {
            self.<a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(key, prop_val);
        } <b>else</b> {
            self.<a href="property_map.md#0x3_property_map_add">add</a>(*key, prop_val);
        };
    }
}
</code></pre>



</details>

<a id="0x3_property_map_update_property_value"></a>

## Function `update_property_value`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(
    self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,
    key: &String,
    value: <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>
) {
    <b>let</b> property_val = self.map.borrow_mut(key);
    *property_val = value;
}
</code></pre>



</details>

<a id="0x3_property_map_create_property_value_raw"></a>

## Function `create_property_value_raw`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(
    value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    type: String
): <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> {
    <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> {
        value,
        type,
    }
}
</code></pre>



</details>

<a id="0x3_property_map_create_property_value"></a>

## Function `create_property_value`

create a property value from generic type data


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &T): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &T): <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> {
    <b>let</b> name = type_name&lt;T&gt;();
    <b>if</b> (
        name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"bool") ||
            name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u8") ||
            name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u64") ||
            name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u128") ||
            name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<b>address</b>") ||
            name == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>")
    ) {
        <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;T&gt;(data), name)
    } <b>else</b> {
        <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;T&gt;(data), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;"))
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
<b>let</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a> = 1000;
<b>let</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a> = 128;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> length = len(keys);
<b>aborts_if</b> !(<a href="property_map.md#0x3_property_map_length">length</a> &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);
<b>aborts_if</b> !(length == len(values));
<b>aborts_if</b> !(length == len(types));
</code></pre>



<a id="@Specification_1_new_with_key_and_property_value"></a>

### Function `new_with_key_and_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> length = len(keys);
<b>aborts_if</b> !(<a href="property_map.md#0x3_property_map_length">length</a> &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);
<b>aborts_if</b> !(length == len(values));
</code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>




<pre><code><b>aborts_if</b> !(key.<a href="property_map.md#0x3_property_map_length">length</a>() &lt;= <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>);
<b>aborts_if</b> !(!<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key));
<b>aborts_if</b> !(<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(self.map) &lt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);
</code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): u64
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>




<pre><code><b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_types"></a>

### Function `types`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_read_string"></a>

### Function `read_string`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>


Check utf8 for correctness and whether equal
to <code>prop.type</code>


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>");
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>");
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;String&gt;(prop.value);
</code></pre>




<a id="0x3_property_map_spec_utf8"></a>


<pre><code><b>fun</b> <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(bytes: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): String {
   String { bytes }
}
</code></pre>



<a id="@Specification_1_read_u8"></a>

### Function `read_u8`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8
</code></pre>




<pre><code><b>let</b> str = b"u8";
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;u8&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_u64"></a>

### Function `read_u64`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>




<pre><code><b>let</b> str = b"u64";
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;u64&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_address"></a>

### Function `read_address`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>




<pre><code><b>let</b> str = b"<b>address</b>";
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;<b>address</b>&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_u128"></a>

### Function `read_u128`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128
</code></pre>




<pre><code><b>let</b> str = b"u128";
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;u128&gt;(prop.value);
</code></pre>



<a id="@Specification_1_read_bool"></a>

### Function `read_bool`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(self: &<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>




<pre><code><b>let</b> str = b"bool";
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);
<b>let</b> prop = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.map, key);
<b>aborts_if</b> prop.type != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);
<b>aborts_if</b> !velor_std::from_bcs::deserializable&lt;bool&gt;(prop.value);
</code></pre>



<a id="@Specification_1_borrow_value"></a>

### Function `borrow_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_type"></a>

### Function `borrow_type`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(self: &<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
</code></pre>



<a id="@Specification_1_update_property_map"></a>

### Function `update_property_map`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> key_len = len(keys);
<b>let</b> val_len = len(values);
<b>let</b> typ_len = len(types);
<b>aborts_if</b> !(key_len == val_len);
<b>aborts_if</b> !(key_len == typ_len);
</code></pre>



<a id="@Specification_1_update_property_value"></a>

### Function `update_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(self: &<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.map, key);
</code></pre>



<a id="@Specification_1_create_property_value_raw"></a>

### Function `create_property_value_raw`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_create_property_value"></a>

### Function `create_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &T): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>
</code></pre>


Abort according to the code


<pre><code><b>let</b> name = type_name&lt;T&gt;();
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"bool");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"u8");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u8") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"u64");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u8") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u64") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"u128");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u8") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u64") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u128") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"<b>address</b>");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u8") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u64") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u128") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"<b>address</b>") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>");
<b>aborts_if</b> name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"bool") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u8") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u64") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"u128") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"<b>address</b>") &&
    name != <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>") &&
    !<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;");
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
