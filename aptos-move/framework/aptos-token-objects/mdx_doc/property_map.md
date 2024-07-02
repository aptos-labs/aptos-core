
<a id="0x4_property_map"></a>

# Module `0x4::property_map`

<code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code> provides generic metadata support for <code>AptosToken</code>. It is a specialization of
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


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x4_property_map_PropertyMap"></a>

## Resource `PropertyMap`

A Map for typed key to value mapping, the contract using it
should keep track of what keys are what types, and parse them accordingly.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> <b>has</b> drop, key<br /></code></pre>



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

<a id="0x4_property_map_PropertyValue"></a>

## Struct `PropertyValue`

A typed value for the <code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code> to ensure that typing is always consistent


<pre><code><b>struct</b> <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_property_map_MutatorRef"></a>

## Struct `MutatorRef`

A mutator ref that allows for mutation of the property map


<pre><code><b>struct</b> <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x4_property_map_ADDRESS"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>: u8 &#61; 7;<br /></code></pre>



<a id="0x4_property_map_BOOL"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>: u8 &#61; 0;<br /></code></pre>



<a id="0x4_property_map_BYTE_VECTOR"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>: u8 &#61; 8;<br /></code></pre>



<a id="0x4_property_map_EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP">EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x4_property_map_EKEY_TYPE_COUNT_MISMATCH"></a>

Property key and type counts do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x4_property_map_EKEY_VALUE_COUNT_MISMATCH"></a>

Property key and value counts do not match


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST"></a>

The property map does not exist


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG"></a>

The key of the property is too long


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG">EPROPERTY_MAP_KEY_TOO_LONG</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x4_property_map_ETOO_MANY_PROPERTIES"></a>

The number of properties exceeds the maximum


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETOO_MANY_PROPERTIES">ETOO_MANY_PROPERTIES</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x4_property_map_ETYPE_INVALID"></a>

Invalid value type specified


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x4_property_map_MAX_PROPERTY_MAP_SIZE"></a>

Maximum number of items in a <code><a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a></code>


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>: u64 &#61; 1000;<br /></code></pre>



<a id="0x4_property_map_MAX_PROPERTY_NAME_LENGTH"></a>

Maximum number of characters in a property name


<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x4_property_map_STRING"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_STRING">STRING</a>: u8 &#61; 9;<br /></code></pre>



<a id="0x4_property_map_U128"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U128">U128</a>: u8 &#61; 5;<br /></code></pre>



<a id="0x4_property_map_U16"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U16">U16</a>: u8 &#61; 2;<br /></code></pre>



<a id="0x4_property_map_U256"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U256">U256</a>: u8 &#61; 6;<br /></code></pre>



<a id="0x4_property_map_U32"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U32">U32</a>: u8 &#61; 3;<br /></code></pre>



<a id="0x4_property_map_U64"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U64">U64</a>: u8 &#61; 4;<br /></code></pre>



<a id="0x4_property_map_U8"></a>



<pre><code><b>const</b> <a href="property_map.md#0x4_property_map_U8">U8</a>: u8 &#61; 1;<br /></code></pre>



<a id="0x4_property_map_init"></a>

## Function `init`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, container: <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_init">init</a>(ref: &amp;ConstructorRef, container: <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>) &#123;<br />    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(ref);<br />    <b>move_to</b>(&amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, container);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_extend"></a>

## Function `extend`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_extend">extend</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a>, container: <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_extend">extend</a>(ref: &amp;ExtendRef, container: <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>) &#123;<br />    <b>let</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(ref);<br />    <b>move_to</b>(&amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, container);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_burn"></a>

## Function `burn`

Burns the entire property map


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_burn">burn</a>(ref: <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_burn">burn</a>(ref: <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>move_from</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_prepare_input"></a>

## Function `prepare_input`

Helper for external entry functions to produce a valid container for property values.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="property_map.md#0x4_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_prepare_input">prepare_input</a>(<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />): <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> length &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_length">length</a> &lt;&#61; <a href="property_map.md#0x4_property_map_MAX_PROPERTY_MAP_SIZE">MAX_PROPERTY_MAP_SIZE</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETOO_MANY_PROPERTIES">ETOO_MANY_PROPERTIES</a>));<br />    <b>assert</b>!(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;values), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_VALUE_COUNT_MISMATCH">EKEY_VALUE_COUNT_MISMATCH</a>));<br />    <b>assert</b>!(length &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;types), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EKEY_TYPE_COUNT_MISMATCH">EKEY_TYPE_COUNT_MISMATCH</a>));<br /><br />    <b>let</b> container &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a>&gt;();<br />    <b>while</b> (!<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;keys)) &#123;<br />        <b>let</b> key &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> keys);<br />        <b>assert</b>!(<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;key) &lt;&#61; <a href="property_map.md#0x4_property_map_MAX_PROPERTY_NAME_LENGTH">MAX_PROPERTY_NAME_LENGTH</a>,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG">EPROPERTY_MAP_KEY_TOO_LONG</a>),<br />        );<br /><br />        <b>let</b> value &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> values);<br />        <b>let</b> type &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> types);<br /><br />        <b>let</b> new_type &#61; <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);<br />        <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);<br /><br />        <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> container, key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> &#123; value, type: new_type &#125;);<br />    &#125;;<br /><br />    <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123; inner: container &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_to_external_type"></a>

## Function `to_external_type`

Maps <code>String</code> representation of types from their <code>u8</code> representation


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(type: u8): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(type: u8): String &#123;<br />    <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;bool&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U8">U8</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u8&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U16">U16</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u16&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U32">U32</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u32&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U64">U64</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u64&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U128">U128</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u128&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U256">U256</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u256&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<b>address</b>&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&quot;)<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_STRING">STRING</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;)<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_to_internal_type"></a>

## Function `to_internal_type`

Maps the <code>String</code> representation of types to <code>u8</code>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type: String): u8 &#123;<br />    <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;bool&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_BOOL">BOOL</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u8&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U8">U8</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u16&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U16">U16</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u32&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U32">U32</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u64&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U64">U64</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u128&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U128">U128</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u256&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_U256">U256</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<b>address</b>&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a><br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;)) &#123;<br />        <a href="property_map.md#0x4_property_map_STRING">STRING</a><br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_INVALID">ETYPE_INVALID</a>))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_type_info_to_internal_type"></a>

## Function `type_info_to_internal_type`

Maps Move type to <code>u8</code> representation


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;(): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;(): u8 &#123;<br />    <b>let</b> type &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();<br />    <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_validate_type"></a>

## Function `validate_type`

Validates property value type against its expected type


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) &#123;<br />    <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_BOOL">BOOL</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U8">U8</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U16">U16</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u16">from_bcs::to_u16</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U32">U32</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u32">from_bcs::to_u32</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U64">U64</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U128">U128</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_U256">U256</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_ADDRESS">ADDRESS</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(value);<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_BYTE_VECTOR">BYTE_VECTOR</a>) &#123;<br />        // nothing <b>to</b> validate...<br />    &#125; <b>else</b> <b>if</b> (type &#61;&#61; <a href="property_map.md#0x4_property_map_STRING">STRING</a>) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(value);<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>))<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;ConstructorRef): <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> &#123;<br />    <a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a> &#123; self: <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(ref) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_contains_key">contains_key</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;<a href="property_map.md#0x4_property_map">property_map</a>.inner, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_length"></a>

## Function `length`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_length">length</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&amp;<a href="property_map.md#0x4_property_map">property_map</a>.inner)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read"></a>

## Function `read`

Read the property and get it&apos;s external type in it&apos;s bcs encoded format

The preferred method is to use <code>read_&lt;type&gt;</code> where the type is already known.


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read">read</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): (String, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>));<br />    <b>let</b> property_value &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;<a href="property_map.md#0x4_property_map">property_map</a>.inner, key);<br />    <b>let</b> new_type &#61; <a href="property_map.md#0x4_property_map_to_external_type">to_external_type</a>(property_value.type);<br />    (new_type, property_value.value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_assert_exists"></a>

## Function `assert_exists`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: <b>address</b>) &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="property_map.md#0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST">EPROPERTY_MAP_DOES_NOT_EXIST</a>),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_typed"></a>

## Function `read_typed`

Read a type and verify that the type is correct


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T: key, V&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T: key, V&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> (type, value) &#61; <a href="property_map.md#0x4_property_map_read">read</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <b>assert</b>!(<br />        type &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;V&gt;(),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="property_map.md#0x4_property_map_ETYPE_MISMATCH">ETYPE_MISMATCH</a>),<br />    );<br />    value<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_bool"></a>

## Function `read_bool`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bool">read_bool</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): bool <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, bool&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u8"></a>

## Function `read_u8`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u8">read_u8</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u8 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u8&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u8">from_bcs::to_u8</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u16"></a>

## Function `read_u16`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u16<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u16">read_u16</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u16 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u16&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u16">from_bcs::to_u16</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u32"></a>

## Function `read_u32`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u32<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u32">read_u32</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u32 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u32&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u32">from_bcs::to_u32</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u64"></a>

## Function `read_u64`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u64">read_u64</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u64 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u64&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u64">from_bcs::to_u64</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u128"></a>

## Function `read_u128`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u128">read_u128</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u128 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u128&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u128">from_bcs::to_u128</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_u256"></a>

## Function `read_u256`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u256<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): u256 <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, u256&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_address"></a>

## Function `read_address`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_address">read_address</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): <b>address</b> <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, <b>address</b>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_bytes"></a>

## Function `read_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bytes">from_bcs::to_bytes</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_read_string"></a>

## Function `read_string`



<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_read_string">read_string</a>&lt;T: key&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: &amp;Object&lt;T&gt;, key: &amp;String): String <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> value &#61; <a href="property_map.md#0x4_property_map_read_typed">read_typed</a>&lt;T, String&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, key);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_string">from_bcs::to_string</a>(value)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_add"></a>

## Function `add`

Add a property, already bcs encoded as a <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add">add</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: String, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> new_type &#61; <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);<br />    <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);<br />    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, new_type, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_add_typed"></a>

## Function `add_typed`

Add a property that isn&apos;t already encoded as a <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_add_typed">add_typed</a>&lt;T: drop&gt;(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> type &#61; <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;();<br />    <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref, key, type, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;value));<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_add_internal">add_internal</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: String, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key, <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> &#123; type, value &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_update"></a>

## Function `update`

Updates a property in place already bcs encoded


<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>update</b>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &amp;String, type: String, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> new_type &#61; <a href="property_map.md#0x4_property_map_to_internal_type">to_internal_type</a>(type);<br />    <a href="property_map.md#0x4_property_map_validate_type">validate_type</a>(new_type, value);<br />    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, new_type, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_update_typed"></a>

## Function `update_typed`

Updates a property in place that is not already bcs encoded


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_update_typed">update_typed</a>&lt;T: drop&gt;(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &amp;String, value: T) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>let</b> type &#61; <a href="property_map.md#0x4_property_map_type_info_to_internal_type">type_info_to_internal_type</a>&lt;T&gt;();<br />    <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref, key, type, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;value));<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_update_internal"></a>

## Function `update_internal`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="property_map.md#0x4_property_map_update_internal">update_internal</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &amp;String, type: u8, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);<br />    <b>let</b> old_value &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key);<br />    &#42;old_value &#61; <a href="property_map.md#0x4_property_map_PropertyValue">PropertyValue</a> &#123; type, value &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_remove"></a>

## Function `remove`

Removes a property from the map, ensuring that it does in fact exist


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a>, key: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="property_map.md#0x4_property_map_remove">remove</a>(ref: &amp;<a href="property_map.md#0x4_property_map_MutatorRef">MutatorRef</a>, key: &amp;String) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <a href="property_map.md#0x4_property_map_assert_exists">assert_exists</a>(ref.self);<br />    <b>let</b> <a href="property_map.md#0x4_property_map">property_map</a> &#61; <b>borrow_global_mut</b>&lt;<a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a>&gt;(ref.self);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&amp;<b>mut</b> <a href="property_map.md#0x4_property_map">property_map</a>.inner, key);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_property_map_assert_end_to_end_input"></a>

## Function `assert_end_to_end_input`



<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_end_to_end_input">assert_end_to_end_input</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/object.md#0x1_object_ObjectCore">object::ObjectCore</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="property_map.md#0x4_property_map_assert_end_to_end_input">assert_end_to_end_input</a>(<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>: Object&lt;ObjectCore&gt;) <b>acquires</b> <a href="property_map.md#0x4_property_map_PropertyMap">PropertyMap</a> &#123;<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_bool">read_bool</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;bool&quot;)), 0);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u8">read_u8</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u8&quot;)) &#61;&#61; 0x12, 1);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u16">read_u16</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u16&quot;)) &#61;&#61; 0x1234, 2);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u32">read_u32</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u32&quot;)) &#61;&#61; 0x12345678, 3);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u64">read_u64</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u64&quot;)) &#61;&#61; 0x1234567812345678, 4);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_u128">read_u128</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u128&quot;)) &#61;&#61; 0x12345678123456781234567812345678, 5);<br />    <b>assert</b>!(<br />        <a href="property_map.md#0x4_property_map_read_u256">read_u256</a>(<br />            &amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>,<br />            &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;u256&quot;)<br />        ) &#61;&#61; 0x1234567812345678123456781234567812345678123456781234567812345678,<br />        6<br />    );<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_bytes">read_bytes</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&quot;)) &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0x01], 7);<br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_read_string">read_string</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">0x1::string::String</a>&quot;)) &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;a&quot;), 8);<br /><br />    <b>assert</b>!(<a href="property_map.md#0x4_property_map_length">length</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) &#61;&#61; 9, 9);<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
