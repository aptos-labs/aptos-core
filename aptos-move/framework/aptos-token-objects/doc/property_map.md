
<a name="0x4_property_map"></a>

# Module `0x4::property_map`

PropertyMap provides generic metadata support for AptosToken. It is a  specialization of
SimpleMap that enforces strict typing with minimal storage use by using constant u64 to
represent types and storing values in bcs format.


-  [Resource `PropertyMap`](#0x4_property_map_PropertyMap)
-  [Struct `PropertyValue`](#0x4_property_map_PropertyValue)
-  [Struct `MutatorRef`](#0x4_property_map_MutatorRef)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x4_property_map_init)
-  [Function `prepare_input`](#0x4_property_map_prepare_input)
-  [Function `generate_mutator_ref`](#0x4_property_map_generate_mutator_ref)
-  [Function `contains_key`](#0x4_property_map_contains_key)
-  [Function `length`](#0x4_property_map_length)
-  [Function `read`](#0x4_property_map_read)
-  [Function `read_bool`](#0x4_property_map_read_bool)
-  [Function `read_u8`](#0x4_property_map_read_u8)
-  [Function `read_u16`](#0x4_property_map_read_u16)
-  [Function `read_u32`](#0x4_property_map_read_u32)
-  [Function `read_u64`](#0x4_property_map_read_u64)
-  [Function `read_u128`](#0x4_property_map_read_u128)
-  [Function `read_u256`](#0x4_property_map_read_u256)
-  [Function `read_address`](#0x4_property_map_read_address)
-  [Function `read_bytes`](#0x4_property_map_read_bytes)
-  [Function `read_string`](#0x4_property_map_read_string)
-  [Function `add`](#0x4_property_map_add)
-  [Function `add_typed`](#0x4_property_map_add_typed)
-  [Function `add_internal`](#0x4_property_map_add_internal)
-  [Function `update`](#0x4_property_map_update)
-  [Function `update_typed`](#0x4_property_map_update_typed)
-  [Function `update_internal`](#0x4_property_map_update_internal)
-  [Function `remove`](#0x4_property_map_remove)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x4_property_map_PropertyMap"></a>

## Resource `PropertyMap`



<pre><code><b>struct</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x4_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_property_map_PropertyValue"></a>

## Struct `PropertyValue`



<pre><code><b>struct</b> <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_property_map_MutatorRef"></a>

## Struct `MutatorRef`



<pre><code><b>struct</b> <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x4_property_map_ETYPE_MISMATCH"></a>

Property type does not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 5;
</code></pre>



<a name="0x4_property_map_ADDRESS"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>: u8 = 7;
</code></pre>



<a name="0x4_property_map_BOOL"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>: u8 = 0;
</code></pre>



<a name="0x4_property_map_BYTE_VECTOR"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>: u8 = 8;
</code></pre>



<a name="0x4_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_AREADY_EXIST_IN_PROPERTY_MAP">EKEY_AREADY_EXIST_IN_PROPERTY_MAP</a>: u64 = 1;
</code></pre>



<a name="0x4_property_map_EKEY_TYPE_COUNT_MISMATCH"></a>

Property key and type count do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>: u64 = 4;
</code></pre>



<a name="0x4_property_map_EKEY_VALUE_COUNT_MISMATCH"></a>

Property key and value counts do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>: u64 = 3;
</code></pre>



<a name="0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST"></a>

The property map does not exist within global storage


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>: u64 = 7;
</code></pre>



<a name="0x4_property_map_EPROPERTY_MAP_NAME_TOO_LONG"></a>

The name (key) of the property is too long


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>: u64 = 6;
</code></pre>



<a name="0x4_property_map_EPROPERTY_NUMBER_EXCEEDS_LIMIT"></a>

The number of property exceeds the limit


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_NUMBER_EXCEEDS_LIMIT">EPROPERTY_NUMBER_EXCEEDS_LIMIT</a>: u64 = 2;
</code></pre>



<a name="0x4_property_map_ETYPE_INVALID"></a>

Invalid type specified


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>: u64 = 8;
</code></pre>



<a name="0x4_property_map_MAX_PROPERTY_MAP_SIZE"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>: u64 = 1000;
</code></pre>



<a name="0x4_property_map_MAX_PROPERTY_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a name="0x4_property_map_STRING"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_STRING">STRING</a>: u8 = 9;
</code></pre>



<a name="0x4_property_map_U128"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U128">U128</a>: u8 = 5;
</code></pre>



<a name="0x4_property_map_U16"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U16">U16</a>: u8 = 2;
</code></pre>



<a name="0x4_property_map_U256"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U256">U256</a>: u8 = 6;
</code></pre>



<a name="0x4_property_map_U32"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U32">U32</a>: u8 = 3;
</code></pre>



<a name="0x4_property_map_U64"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U64">U64</a>: u8 = 4;
</code></pre>



<a name="0x4_property_map_U8"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U8">U8</a>: u8 = 1;
</code></pre>



<a name="0x4_property_map_init"></a>

## Function `init`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, container: <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &ConstructorRef, container: <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>) {
    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(ref);
    <b>move_to</b>(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, container);
}
</code></pre>



</details>

<a name="0x4_property_map_prepare_input"></a>

## Function `prepare_input`

Helper for external entry functions to produce a valid container for property values.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(
    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> length = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&keys);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_length">length</a> &lt;= <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_NUMBER_EXCEEDS_LIMIT">EPROPERTY_NUMBER_EXCEEDS_LIMIT</a>));
    <b>assert</b>!(length == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&values), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>));
    <b>assert</b>!(length == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&types), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>));

    <b>let</b> container = <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a>&gt;();
    <b>while</b> (!<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&keys)) {
        <b>let</b> key = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> keys);
        <b>assert</b>!(
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&key) &lt;= <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_NAME_TOO_LONG">EPROPERTY_MAP_NAME_TOO_LONG</a>),
        );

        <b>let</b> value = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> values);
        <b>let</b> type = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> types);

        <b>let</b> new_type = to_internal_type(type);
        validate_type(new_type, value);

        <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> container, key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { value, type: new_type });
    };

    <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> { inner: container }
}
</code></pre>



</details>

<a name="0x4_property_map_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &ConstructorRef): <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> {
    <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> { self: <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(ref) }
}
</code></pre>



</details>

<a name="0x4_property_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>)),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>),
    );
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));
    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&<a href="property_map.md#0x4_property_map">property_map</a>.inner, key)
}
</code></pre>



</details>

<a name="0x4_property_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>)),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>),
    );
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));
    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&<a href="property_map.md#0x4_property_map">property_map</a>.inner)
}
</code></pre>



</details>

<a name="0x4_property_map_read"></a>

## Function `read`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): (String, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>)),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>),
    );
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));
    <b>let</b> property_value = <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&<a href="property_map.md#0x4_property_map">property_map</a>.inner, key);
    <b>let</b> new_type = to_external_type(property_value.type);
    (new_type, property_value.value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_bool"></a>

## Function `read_bool`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, bool&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u8"></a>

## Function `read_u8`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u8 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u8&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u16"></a>

## Function `read_u16`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u16 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u16&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u16">from_bcs::to_u16</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u32"></a>

## Function `read_u32`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u32 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u32&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u32">from_bcs::to_u32</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u64"></a>

## Function `read_u64`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u64&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u128"></a>

## Function `read_u128`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u128 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u128&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_u256"></a>

## Function `read_u256`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u256 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, u256&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_address"></a>

## Function `read_address`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): <b>address</b> <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, <b>address</b>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_bytes"></a>

## Function `read_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bytes">from_bcs::to_bytes</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_read_string"></a>

## Function `read_string`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): String <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = read_typed&lt;T, String&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(value)
}
</code></pre>



</details>

<a name="0x4_property_map_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: String, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> new_type = to_internal_type(type);
    validate_type(new_type, value);
    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, new_type, value);
}
</code></pre>



</details>

<a name="0x4_property_map_add_typed"></a>

## Function `add_typed`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> type = type_info_to_internal_type&lt;T&gt;();
    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, type, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&value));
}
</code></pre>



</details>

<a name="0x4_property_map_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);
    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { type, value });
}
</code></pre>



</details>

<a name="0x4_property_map_update"></a>

## Function `update`



<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, type: String, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> new_type = to_internal_type(type);
    validate_type(new_type, value);
    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, new_type, value);
}
</code></pre>



</details>

<a name="0x4_property_map_update_typed"></a>

## Function `update_typed`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> type = type_info_to_internal_type&lt;T&gt;();
    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, type, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&value));
}
</code></pre>



</details>

<a name="0x4_property_map_update_internal"></a>

## Function `update_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);
    <b>let</b> old_value = <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key);
    *old_value = <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { type, value };
}
</code></pre>



</details>

<a name="0x4_property_map_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);
    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key);
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
