
<a id="0x4_property_map"></a>

# Module `0x4::property_map`

<code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code> provides generic metadata support for <code>VelorToken</code>. It is a specialization of
<code>SimpleMap</code> that enforces strict typing with minimal storage use by using constant u64 to
represent types and storing values in bcs format.


-  [Resource `PropertyMap`](#0x4_property_map_PropertyMap)
-  [Struct `PropertyValue`](#0x4_property_map_PropertyValue)
-  [Struct `MutatorRef`](#0x4_property_map_MutatorRef)
-  [Constants](#@Constants_0)
-  [Function `init`](#0x4_property_map_init)
-  [Function `extend`](#0x4_property_map_extend)
-  [Function `burn`](#0x4_property_map_burn)
-  [Function `prepare_input`](#0x4_property_map_prepare_input)
-  [Function `to_external_type`](#0x4_property_map_to_external_type)
-  [Function `to_internal_type`](#0x4_property_map_to_internal_type)
-  [Function `type_info_to_internal_type`](#0x4_property_map_type_info_to_internal_type)
-  [Function `validate_type`](#0x4_property_map_validate_type)
-  [Function `generate_mutator_ref`](#0x4_property_map_generate_mutator_ref)
-  [Function `contains_key`](#0x4_property_map_contains_key)
-  [Function `length`](#0x4_property_map_length)
-  [Function `read`](#0x4_property_map_read)
-  [Function `assert_exists`](#0x4_property_map_assert_exists)
-  [Function `read_typed`](#0x4_property_map_read_typed)
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
-  [Function `assert_end_to_end_input`](#0x4_property_map_assert_end_to_end_input)


<pre><code><b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../velor-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x4_property_map_PropertyMap"></a>

## Resource `PropertyMap`

A Map for typed key to value mapping, the contract using it
should keep track of what keys are what types, and parse them accordingly.


<pre><code>#[resource_group_member(#[group = <a href="../../velor-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="property_map.md#0x4_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_property_map_PropertyValue"></a>

## Struct `PropertyValue`

A typed value for the <code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code> to ensure that typing is always consistent


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
<code>value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_property_map_MutatorRef"></a>

## Struct `MutatorRef`

A mutator ref that allows for mutation of the property map


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

<a id="@Constants_0"></a>

## Constants


<a id="0x4_property_map_ETYPE_MISMATCH"></a>

Property value does not match expected type


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x4_property_map_ADDRESS"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>: u8 = 7;
</code></pre>



<a id="0x4_property_map_BOOL"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>: u8 = 0;
</code></pre>



<a id="0x4_property_map_BYTE_VECTOR"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>: u8 = 8;
</code></pre>



<a id="0x4_property_map_EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP">EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP</a>: u64 = 2;
</code></pre>



<a id="0x4_property_map_EKEY_TYPE_COUNT_MISMATCH"></a>

Property key and type counts do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>: u64 = 5;
</code></pre>



<a id="0x4_property_map_EKEY_VALUE_COUNT_MISMATCH"></a>

Property key and value counts do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST"></a>

The property map does not exist


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG"></a>

The key of the property is too long


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG">EPROPERTY_MAP_KEY_TOO_LONG</a>: u64 = 8;
</code></pre>



<a id="0x4_property_map_ETOO_MANY_PROPERTIES"></a>

The number of properties exceeds the maximum


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETOO_MANY_PROPERTIES">ETOO_MANY_PROPERTIES</a>: u64 = 3;
</code></pre>



<a id="0x4_property_map_ETYPE_INVALID"></a>

Invalid value type specified


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>: u64 = 7;
</code></pre>



<a id="0x4_property_map_MAX_PROPERTY_MAP_SIZE"></a>

Maximum number of items in a <code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code>


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>: u64 = 1000;
</code></pre>



<a id="0x4_property_map_MAX_PROPERTY_NAME_LENGTH"></a>

Maximum number of characters in a property name


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a id="0x4_property_map_STRING"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_STRING">STRING</a>: u8 = 9;
</code></pre>



<a id="0x4_property_map_U128"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U128">U128</a>: u8 = 5;
</code></pre>



<a id="0x4_property_map_U16"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U16">U16</a>: u8 = 2;
</code></pre>



<a id="0x4_property_map_U256"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U256">U256</a>: u8 = 6;
</code></pre>



<a id="0x4_property_map_U32"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U32">U32</a>: u8 = 3;
</code></pre>



<a id="0x4_property_map_U64"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U64">U64</a>: u8 = 4;
</code></pre>



<a id="0x4_property_map_U8"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U8">U8</a>: u8 = 1;
</code></pre>



<a id="0x4_property_map_init"></a>

## Function `init`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &<a href="../../velor-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, container: <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &ConstructorRef, container: <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>) {
    <b>let</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="../../velor-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(ref);
    <b>move_to</b>(&<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, container);
}
</code></pre>



</details>

<a id="0x4_property_map_extend"></a>

## Function `extend`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_extend">extend</a>(ref: &<a href="../../velor-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a>, container: <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_extend">extend</a>(ref: &ExtendRef, container: <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>) {
    <b>let</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="../../velor-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(ref);
    <b>move_to</b>(&<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, container);
}
</code></pre>



</details>

<a id="0x4_property_map_burn"></a>

## Function `burn`

Burns the entire property map


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_burn">burn</a>(ref: <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_burn">burn</a>(ref: <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>move_from</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);
}
</code></pre>



</details>

<a id="0x4_property_map_prepare_input"></a>

## Function `prepare_input`

Helper for external entry functions to produce a valid container for property values.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(
    keys: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    types: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> length = keys.<a href="property_map.md#0x4_property_map_length">length</a>();
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_length">length</a> &lt;= <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETOO_MANY_PROPERTIES">ETOO_MANY_PROPERTIES</a>));
    <b>assert</b>!(length == values.<a href="property_map.md#0x4_property_map_length">length</a>(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>));
    <b>assert</b>!(length == types.<a href="property_map.md#0x4_property_map_length">length</a>(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>));

    <b>let</b> container = <a href="../../velor-framework/../velor-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a>&gt;();
    <b>while</b> (!keys.is_empty()) {
        <b>let</b> key = keys.pop_back();
        <b>assert</b>!(
            key.<a href="property_map.md#0x4_property_map_length">length</a>() &lt;= <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>,
            <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG">EPROPERTY_MAP_KEY_TOO_LONG</a>),
        );

        <b>let</b> value = values.pop_back();
        <b>let</b> type = types.pop_back();

        <b>let</b> new_type = <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);
        <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);

        container.<a href="property_map.md#0x4_property_map_add">add</a>(key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { value, type: new_type });
    };

    <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> { inner: container }
}
</code></pre>



</details>

<a id="0x4_property_map_to_external_type"></a>

## Function `to_external_type`

Maps <code>String</code> representation of types from their <code>u8</code> representation


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(type: u8): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(type: u8): String {
    <b>if</b> (type == <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"bool")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U8">U8</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u8")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U16">U16</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u16")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U32">U32</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u32")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U64">U64</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u64")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U128">U128</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u128")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U256">U256</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u256")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<b>address</b>")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;")
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_STRING">STRING</a>) {
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>")
    } <b>else</b> {
        <b>abort</b> (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>))
    }
}
</code></pre>



</details>

<a id="0x4_property_map_to_internal_type"></a>

## Function `to_internal_type`

Maps the <code>String</code> representation of types to <code>u8</code>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type: String): u8 {
    <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"bool")) {
        <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u8")) {
        <a href="property_map.md#0x4_property_map_U8">U8</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u16")) {
        <a href="property_map.md#0x4_property_map_U16">U16</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u32")) {
        <a href="property_map.md#0x4_property_map_U32">U32</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u64")) {
        <a href="property_map.md#0x4_property_map_U64">U64</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u128")) {
        <a href="property_map.md#0x4_property_map_U128">U128</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u256")) {
        <a href="property_map.md#0x4_property_map_U256">U256</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<b>address</b>")) {
        <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;")) {
        <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>
    } <b>else</b> <b>if</b> (type == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>")) {
        <a href="property_map.md#0x4_property_map_STRING">STRING</a>
    } <b>else</b> {
        <b>abort</b> (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>))
    }
}
</code></pre>



</details>

<a id="0x4_property_map_type_info_to_internal_type"></a>

## Function `type_info_to_internal_type`

Maps Move type to <code>u8</code> representation


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;(): u8 {
    <b>let</b> type = <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type)
}
</code></pre>



</details>

<a id="0x4_property_map_validate_type"></a>

## Function `validate_type`

Validates property value type against its expected type


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>if</b> (type == <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U8">U8</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U16">U16</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u16">from_bcs::to_u16</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U32">U32</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u32">from_bcs::to_u32</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U64">U64</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U128">U128</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_U256">U256</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(value);
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>) {
        // nothing <b>to</b> validate...
    } <b>else</b> <b>if</b> (type == <a href="property_map.md#0x4_property_map_STRING">STRING</a>) {
        <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(value);
    } <b>else</b> {
        <b>abort</b> (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>))
    };
}
</code></pre>



</details>

<a id="0x4_property_map_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &<a href="../../velor-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &ConstructorRef): <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> {
    <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> { self: <a href="../../velor-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(ref) }
}
</code></pre>



</details>

<a id="0x4_property_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>)];
    <a href="property_map.md#0x4_property_map">property_map</a>.inner.<a href="property_map.md#0x4_property_map_contains_key">contains_key</a>(key)
}
</code></pre>



</details>

<a id="0x4_property_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>)];
    <a href="property_map.md#0x4_property_map">property_map</a>.inner.<a href="property_map.md#0x4_property_map_length">length</a>()
}
</code></pre>



</details>

<a id="0x4_property_map_read"></a>

## Function `read`

Read the property and get it's external type in it's bcs encoded format

The preferred method is to use <code>read_&lt;type&gt;</code> where the type is already known.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): (String, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>));
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[<a href="../../velor-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>)];
    <b>let</b> property_value = <a href="property_map.md#0x4_property_map">property_map</a>.inner.borrow(key);
    <b>let</b> new_type = <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(property_value.type);
    (new_type, property_value.value)
}
</code></pre>



</details>

<a id="0x4_property_map_assert_exists"></a>

## Function `assert_exists`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: <b>address</b>) {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>),
    );
}
</code></pre>



</details>

<a id="0x4_property_map_read_typed"></a>

## Function `read_typed`

Read a type and verify that the type is correct


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T: key, V&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T: key, V&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> (type, value) = <a href="property_map.md#0x4_property_map_read">read</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <b>assert</b>!(
        type == <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;V&gt;(),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>),
    );
    value
}
</code></pre>



</details>

<a id="0x4_property_map_read_bool"></a>

## Function `read_bool`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, bool&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u8"></a>

## Function `read_u8`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u8 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u8&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u16"></a>

## Function `read_u16`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u16
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u16 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u16&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u16">from_bcs::to_u16</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u32"></a>

## Function `read_u32`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u32 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u32&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u32">from_bcs::to_u32</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u64"></a>

## Function `read_u64`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u64&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u128"></a>

## Function `read_u128`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u128 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u128&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_u256"></a>

## Function `read_u256`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): u256 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u256&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_address"></a>

## Function `read_address`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): <b>address</b> <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, <b>address</b>&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_bytes"></a>

## Function `read_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bytes">from_bcs::to_bytes</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_read_string"></a>

## Function `read_string`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &<a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: &Object&lt;T&gt;, key: &String): String <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> value = <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, String&gt;(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, key);
    <a href="../../velor-framework/../velor-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(value)
}
</code></pre>



</details>

<a id="0x4_property_map_add"></a>

## Function `add`

Add a property, already bcs encoded as a <code><a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: String, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> new_type = <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);
    <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);
    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, new_type, value);
}
</code></pre>



</details>

<a id="0x4_property_map_add_typed"></a>

## Function `add_typed`

Add a property that isn't already encoded as a <code><a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> type = <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;();
    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, type, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&value));
}
</code></pre>



</details>

<a id="0x4_property_map_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<b>mut</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[ref.self];
    <a href="property_map.md#0x4_property_map">property_map</a>.inner.<a href="property_map.md#0x4_property_map_add">add</a>(key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { type, value });
}
</code></pre>



</details>

<a id="0x4_property_map_update"></a>

## Function `update`

Updates a property in place already bcs encoded


<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, type: String, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> new_type = <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);
    <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);
    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, new_type, value);
}
</code></pre>



</details>

<a id="0x4_property_map_update_typed"></a>

## Function `update_typed`

Updates a property in place that is not already bcs encoded


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>let</b> type = <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;();
    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, type, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&value));
}
</code></pre>



</details>

<a id="0x4_property_map_update_internal"></a>

## Function `update_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String, type: u8, value: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<b>mut</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[ref.self];
    <b>let</b> old_value = <a href="property_map.md#0x4_property_map">property_map</a>.inner.borrow_mut(key);
    *old_value = <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> { type, value };
}
</code></pre>



</details>

<a id="0x4_property_map_remove"></a>

## Function `remove`

Removes a property from the map, ensuring that it does in fact exist


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &String) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);
    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> = &<b>mut</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>[ref.self];
    <a href="property_map.md#0x4_property_map">property_map</a>.inner.<a href="property_map.md#0x4_property_map_remove">remove</a>(key);
}
</code></pre>



</details>

<a id="0x4_property_map_assert_end_to_end_input"></a>

## Function `assert_end_to_end_input`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_end_to_end_input">assert_end_to_end_input</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: <a href="../../velor-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../velor-framework/doc/object.md#0x1_object_ObjectCore">object::ObjectCore</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_end_to_end_input">assert_end_to_end_input</a>(<a href="../../velor-framework/doc/object.md#0x1_object">object</a>: Object&lt;ObjectCore&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> {
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_bool">read_bool</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"bool")), 0);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u8">read_u8</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u8")) == 0x12, 1);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u16">read_u16</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u16")) == 0x1234, 2);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u32">read_u32</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u32")) == 0x12345678, 3);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u64">read_u64</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u64")) == 0x1234567812345678, 4);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u128">read_u128</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u128")) == 0x12345678123456781234567812345678, 5);
    <b>assert</b>!(
        <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>(
            &<a href="../../velor-framework/doc/object.md#0x1_object">object</a>,
            &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"u256")
        ) == 0x1234567812345678123456781234567812345678123456781234567812345678,
        6
    );
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;")) == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0x01], 7);
    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_string">read_string</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>, &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>")) == <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"a"), 8);

    <b>assert</b>!(<a href="property_map.md#0x4_property_map_length">length</a>(&<a href="../../velor-framework/doc/object.md#0x1_object">object</a>) == 9, 9);
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
