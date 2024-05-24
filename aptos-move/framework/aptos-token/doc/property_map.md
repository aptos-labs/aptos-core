
<a id="0x3_property_map"></a>

# Module `0x3::property_map`

PropertyMap is a specialization of SimpleMap for Tokens.
It maps a String key to a PropertyValue that consists of type (string) and value (vector&lt;u8&gt;)
It provides basic on&#45;chain serialization of primitive and string to property value with type information
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


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;<br /></code></pre>



<a id="0x3_property_map_PropertyMap"></a>

## Struct `PropertyMap`



<pre><code><b>struct</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>map: <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_property_map_PropertyValue"></a>

## Struct `PropertyValue`



<pre><code><b>struct</b> <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP">EKEY_AREADY_EXIST_IN_PROPERTY_MAP</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT"></a>

Property key and type count don&apos;t match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT"></a>

Property key and value count don&apos;t match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG"></a>

The name (key) of the property is too long


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x3_property_map_EPROPERTY_NOT_EXIST"></a>

The property doesn&apos;t exist


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT"></a>

The number of property exceeds the limit


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x3_property_map_ETYPE_NOT_MATCH"></a>

Property type doesn&apos;t match


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x3_property_map_MAX_PROPERTY_MAP_SIZE"></a>

The maximal number of property that can be stored in property map


<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>: u64 &#61; 1000;<br /></code></pre>



<a id="0x3_property_map_MAX_PROPERTY_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x3_property_map_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;<br />): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> length &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys);<br />    <b>assert</b>!(<a href="property_map.md#0x3_property_map_length">length</a> &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));<br />    <b>assert</b>!(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;values), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));<br />    <b>assert</b>!(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;types), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>));<br /><br />    <b>let</b> properties &#61; <a href="property_map.md#0x3_property_map_empty">empty</a>();<br /><br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; length) &#123;<br />        <b>let</b> key &#61; &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;keys, i);<br />        <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;key) &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(<br />            &amp;<b>mut</b> properties.map,<br />            key,<br />            <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123; value: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;values, i), type: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;types, i) &#125;<br />        );<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    properties<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_new_with_key_and_property_value"></a>

## Function `new_with_key_and_property_value`

Create property map directly from key and property value


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>&gt;<br />): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> length &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys);<br />    <b>assert</b>!(<a href="property_map.md#0x3_property_map_length">length</a> &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));<br />    <b>assert</b>!(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;values), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));<br /><br />    <b>let</b> properties &#61; <a href="property_map.md#0x3_property_map_empty">empty</a>();<br /><br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; length) &#123;<br />        <b>let</b> key &#61; &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;keys, i);<br />        <b>let</b> val &#61; &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;values, i);<br />        <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;key) &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));<br />        <a href="property_map.md#0x3_property_map_add">add</a>(&amp;<b>mut</b> properties, key, val);<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    properties<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_empty"></a>

## Function `empty`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a> &#123;<br />        map: <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>&gt;(),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): bool &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;map.map, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: String, value: <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>) &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;key) &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&amp;map.map) &lt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NUMBER_EXCEED_LIMIT">EPROPERTY_NUMBER_EXCEED_LIMIT</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> map.map, key, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): u64 &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&amp;map.map)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): &amp;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123;<br />    <b>let</b> found &#61; <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map, key);<br />    <b>assert</b>!(found, <a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;map.map, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_keys"></a>

## Function `keys`

Return all the keys in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&amp;map.map)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_types"></a>

## Function `types`

Return the types of all properties in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&amp;<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_values">simple_map::values</a>(&amp;map.map), &#124;v&#124; &#123;<br />        <b>let</b> v: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#61; v;<br />        v.type<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_values"></a>

## Function `values`

Return the values of all properties in the property map in the order they are added.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&amp;<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_values">simple_map::values</a>(&amp;map.map), &#124;v&#124; &#123;<br />        <b>let</b> v: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#61; v;<br />        v.value<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_string"></a>

## Function `read_string`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): String &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_u8"></a>

## Function `read_u8`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): u8 &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u8&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_u64"></a>

## Function `read_u64`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): u64 &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u64&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_address"></a>

## Function `read_address`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): <b>address</b> &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<b>address</b>&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_u128"></a>

## Function `read_u128`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): u128 &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u128&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_read_bool"></a>

## Function `read_bool`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>, key: &amp;String): bool &#123;<br />    <b>let</b> prop &#61; <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map, key);<br />    <b>assert</b>!(prop.type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;bool&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_ETYPE_NOT_MATCH">ETYPE_NOT_MATCH</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(prop.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_borrow_value"></a>

## Function `borrow_value`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    property.value<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_borrow_type"></a>

## Function `borrow_type`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>): String &#123;<br />    property.type<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(<br />    map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,<br />    key: &amp;String<br />): (String, <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a>) &#123;<br />    <b>let</b> found &#61; <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map, key);<br />    <b>assert</b>!(found, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x3_property_map_EPROPERTY_NOT_EXIST">EPROPERTY_NOT_EXIST</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&amp;<b>mut</b> map.map, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_update_property_map"></a>

## Function `update_property_map`

Update the property in the existing property map
Allow updating existing keys&apos; value and add new key&#45;value pairs


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(<br />    map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />) &#123;<br />    <b>let</b> key_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys);<br />    <b>let</b> val_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;values);<br />    <b>let</b> typ_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;types);<br />    <b>assert</b>!(key_len &#61;&#61; val_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_VALUE_COUNT">EKEY_COUNT_NOT_MATCH_VALUE_COUNT</a>));<br />    <b>assert</b>!(key_len &#61;&#61; typ_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="property_map.md#0x3_property_map_EKEY_COUNT_NOT_MATCH_TYPE_COUNT">EKEY_COUNT_NOT_MATCH_TYPE_COUNT</a>));<br /><br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; key_len) &#123;<br />        <b>let</b> key &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;keys, i);<br />        <b>let</b> prop_val &#61; <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123;<br />            value: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;values, i),<br />            type: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;types, i),<br />        &#125;;<br />        <b>if</b> (<a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map, key)) &#123;<br />            <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(map, key, prop_val);<br />        &#125; <b>else</b> &#123;<br />            <a href="property_map.md#0x3_property_map_add">add</a>(map, &#42;key, prop_val);<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_update_property_value"></a>

## Function `update_property_value`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(<br />    map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">PropertyMap</a>,<br />    key: &amp;String,<br />    value: <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a><br />) &#123;<br />    <b>let</b> property_val &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> map.map, key);<br />    &#42;property_val &#61; value;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_create_property_value_raw"></a>

## Function `create_property_value_raw`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(<br />    value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    type: String<br />): <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123;<br />    <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123;<br />        value,<br />        type,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_property_map_create_property_value"></a>

## Function `create_property_value`

create a property value from generic type data


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &amp;T): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &amp;T): <a href="property_map.md#0x3_property_map_PropertyValue">PropertyValue</a> &#123;<br />    <b>let</b> name &#61; type_name&lt;T&gt;();<br />    <b>if</b> (<br />        name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;bool&quot;) &#124;&#124;<br />            name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u8&quot;) &#124;&#124;<br />            name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u64&quot;) &#124;&#124;<br />            name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u128&quot;) &#124;&#124;<br />            name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<b>address</b>&quot;) &#124;&#124;<br />            name &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;)<br />    ) &#123;<br />        <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;T&gt;(data), name)<br />    &#125; <b>else</b> &#123;<br />        <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;T&gt;(data), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&quot;))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /><b>let</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a> &#61; 1000;<br /><b>let</b> <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>  &#61; 128;<br /></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new">new</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> length &#61; len(keys);<br /><b>aborts_if</b> !(<a href="property_map.md#0x3_property_map_length">length</a> &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);<br /><b>aborts_if</b> !(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(values));<br /><b>aborts_if</b> !(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(types));<br /></code></pre>



<a id="@Specification_1_new_with_key_and_property_value"></a>

### Function `new_with_key_and_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_new_with_key_and_property_value">new_with_key_and_property_value</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> length &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(keys);<br /><b>aborts_if</b> !(<a href="property_map.md#0x3_property_map_length">length</a> &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);<br /><b>aborts_if</b> !(length &#61;&#61; len(values));<br /></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_empty">empty</a>(): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_contains_key"></a>

### Function `contains_key`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_contains_key">contains_key</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_add">add</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> !(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(key) &lt;&#61; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>);<br /><b>aborts_if</b> !(!<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key));<br /><b>aborts_if</b> !(<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(map.map) &lt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>);<br /></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_length">length</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow">borrow</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /></code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_keys">keys</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_types"></a>

### Function `types`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_types">types</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_values">values</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_read_string"></a>

### Function `read_string`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_string">read_string</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>


Check utf8 for correctness and whether equal
to <code>prop.type</code>


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;String&gt;(prop.value);<br /></code></pre>




<a id="0x3_property_map_spec_utf8"></a>


<pre><code><b>fun</b> <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): String &#123;<br />   String&#123;bytes&#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_read_u8"></a>

### Function `read_u8`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u8">read_u8</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8<br /></code></pre>




<pre><code><b>let</b> str &#61; b&quot;u8&quot;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;u8&gt;(prop.value);<br /></code></pre>



<a id="@Specification_1_read_u64"></a>

### Function `read_u64`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u64">read_u64</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>




<pre><code><b>let</b> str &#61; b&quot;u64&quot;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;u64&gt;(prop.value);<br /></code></pre>



<a id="@Specification_1_read_address"></a>

### Function `read_address`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_address">read_address</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>




<pre><code><b>let</b> str &#61; b&quot;<b>address</b>&quot;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;<b>address</b>&gt;(prop.value);<br /></code></pre>



<a id="@Specification_1_read_u128"></a>

### Function `read_u128`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_u128">read_u128</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128<br /></code></pre>




<pre><code><b>let</b> str &#61; b&quot;u128&quot;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;u128&gt;(prop.value);<br /></code></pre>



<a id="@Specification_1_read_bool"></a>

### Function `read_bool`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_read_bool">read_bool</a>(map: &amp;<a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>




<pre><code><b>let</b> str &#61; b&quot;bool&quot;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(str);<br /><b>let</b> prop &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(map.map, key);<br /><b>aborts_if</b> prop.type !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(str);<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;bool&gt;(prop.value);<br /></code></pre>



<a id="@Specification_1_borrow_value"></a>

### Function `borrow_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_value">borrow_value</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_borrow_type"></a>

### Function `borrow_type`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_borrow_type">borrow_type</a>(property: &amp;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_remove">remove</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /></code></pre>



<a id="@Specification_1_update_property_map"></a>

### Function `update_property_map`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_map">update_property_map</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> key_len &#61; len(keys);<br /><b>let</b> val_len &#61; len(values);<br /><b>let</b> typ_len &#61; len(types);<br /><b>aborts_if</b> !(key_len &#61;&#61; val_len);<br /><b>aborts_if</b> !(key_len &#61;&#61; typ_len);<br /></code></pre>



<a id="@Specification_1_update_property_value"></a>

### Function `update_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_update_property_value">update_property_value</a>(map: &amp;<b>mut</b> <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(map.map, key);<br /></code></pre>



<a id="@Specification_1_create_property_value_raw"></a>

### Function `create_property_value_raw`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value_raw">create_property_value_raw</a>(value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_create_property_value"></a>

### Function `create_property_value`


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x3_property_map_create_property_value">create_property_value</a>&lt;T: <b>copy</b>&gt;(data: &amp;T): <a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a><br /></code></pre>


Abort according to the code


<pre><code><b>let</b> name &#61; type_name&lt;T&gt;();<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;bool&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;u8&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u8&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;u64&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u8&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u64&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;u128&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u8&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u64&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u128&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;<b>address</b>&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u8&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u64&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u128&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;<b>address</b>&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;);<br /><b>aborts_if</b> name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;bool&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u8&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u64&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;u128&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;<b>address</b>&quot;) &amp;&amp;<br />    name !&#61; <a href="property_map.md#0x3_property_map_spec_utf8">spec_utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;) &amp;&amp;<br />    !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&quot;);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
