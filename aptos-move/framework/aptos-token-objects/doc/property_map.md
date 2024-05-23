
<a id="0x4_property_map"></a>

# Module `0x4::property_map`

<code>PropertyMap</code> provides generic metadata support for <code>AptosToken</code>. It is a specialization of<br/> <code>SimpleMap</code> that enforces strict typing with minimal storage use by using constant u64 to<br/> represent types and storing values in bcs format.


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


<pre><code>use 0x1::bcs;<br/>use 0x1::error;<br/>use 0x1::from_bcs;<br/>use 0x1::object;<br/>use 0x1::simple_map;<br/>use 0x1::string;<br/>use 0x1::type_info;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x4_property_map_PropertyMap"></a>

## Resource `PropertyMap`

A Map for typed key to value mapping, the contract using it<br/> should keep track of what keys are what types, and parse them accordingly.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct PropertyMap has drop, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: simple_map::SimpleMap&lt;string::String, property_map::PropertyValue&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_property_map_PropertyValue"></a>

## Struct `PropertyValue`

A typed value for the <code>PropertyMap</code> to ensure that typing is always consistent


<pre><code>struct PropertyValue has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>value: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_property_map_MutatorRef"></a>

## Struct `MutatorRef`

A mutator ref that allows for mutation of the property map


<pre><code>struct MutatorRef has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x4_property_map_ETYPE_MISMATCH"></a>

Property value does not match expected type


<pre><code>const ETYPE_MISMATCH: u64 &#61; 6;<br/></code></pre>



<a id="0x4_property_map_ADDRESS"></a>



<pre><code>const ADDRESS: u8 &#61; 7;<br/></code></pre>



<a id="0x4_property_map_BOOL"></a>



<pre><code>const BOOL: u8 &#61; 0;<br/></code></pre>



<a id="0x4_property_map_BYTE_VECTOR"></a>



<pre><code>const BYTE_VECTOR: u8 &#61; 8;<br/></code></pre>



<a id="0x4_property_map_EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP"></a>

The property key already exists


<pre><code>const EKEY_ALREADY_EXISTS_IN_PROPERTY_MAP: u64 &#61; 2;<br/></code></pre>



<a id="0x4_property_map_EKEY_TYPE_COUNT_MISMATCH"></a>

Property key and type counts do not match


<pre><code>const EKEY_TYPE_COUNT_MISMATCH: u64 &#61; 5;<br/></code></pre>



<a id="0x4_property_map_EKEY_VALUE_COUNT_MISMATCH"></a>

Property key and value counts do not match


<pre><code>const EKEY_VALUE_COUNT_MISMATCH: u64 &#61; 4;<br/></code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_DOES_NOT_EXIST"></a>

The property map does not exist


<pre><code>const EPROPERTY_MAP_DOES_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x4_property_map_EPROPERTY_MAP_KEY_TOO_LONG"></a>

The key of the property is too long


<pre><code>const EPROPERTY_MAP_KEY_TOO_LONG: u64 &#61; 8;<br/></code></pre>



<a id="0x4_property_map_ETOO_MANY_PROPERTIES"></a>

The number of properties exceeds the maximum


<pre><code>const ETOO_MANY_PROPERTIES: u64 &#61; 3;<br/></code></pre>



<a id="0x4_property_map_ETYPE_INVALID"></a>

Invalid value type specified


<pre><code>const ETYPE_INVALID: u64 &#61; 7;<br/></code></pre>



<a id="0x4_property_map_MAX_PROPERTY_MAP_SIZE"></a>

Maximum number of items in a <code>PropertyMap</code>


<pre><code>const MAX_PROPERTY_MAP_SIZE: u64 &#61; 1000;<br/></code></pre>



<a id="0x4_property_map_MAX_PROPERTY_NAME_LENGTH"></a>

Maximum number of characters in a property name


<pre><code>const MAX_PROPERTY_NAME_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x4_property_map_STRING"></a>



<pre><code>const STRING: u8 &#61; 9;<br/></code></pre>



<a id="0x4_property_map_U128"></a>



<pre><code>const U128: u8 &#61; 5;<br/></code></pre>



<a id="0x4_property_map_U16"></a>



<pre><code>const U16: u8 &#61; 2;<br/></code></pre>



<a id="0x4_property_map_U256"></a>



<pre><code>const U256: u8 &#61; 6;<br/></code></pre>



<a id="0x4_property_map_U32"></a>



<pre><code>const U32: u8 &#61; 3;<br/></code></pre>



<a id="0x4_property_map_U64"></a>



<pre><code>const U64: u8 &#61; 4;<br/></code></pre>



<a id="0x4_property_map_U8"></a>



<pre><code>const U8: u8 &#61; 1;<br/></code></pre>



<a id="0x4_property_map_init"></a>

## Function `init`



<pre><code>public fun init(ref: &amp;object::ConstructorRef, container: property_map::PropertyMap)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun init(ref: &amp;ConstructorRef, container: PropertyMap) &#123;<br/>    let signer &#61; object::generate_signer(ref);<br/>    move_to(&amp;signer, container);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_extend"></a>

## Function `extend`



<pre><code>public fun extend(ref: &amp;object::ExtendRef, container: property_map::PropertyMap)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extend(ref: &amp;ExtendRef, container: PropertyMap) &#123;<br/>    let signer &#61; object::generate_signer_for_extending(ref);<br/>    move_to(&amp;signer, container);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_burn"></a>

## Function `burn`

Burns the entire property map


<pre><code>public fun burn(ref: property_map::MutatorRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn(ref: MutatorRef) acquires PropertyMap &#123;<br/>    move_from&lt;PropertyMap&gt;(ref.self);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_prepare_input"></a>

## Function `prepare_input`

Helper for external entry functions to produce a valid container for property values.


<pre><code>public fun prepare_input(keys: vector&lt;string::String&gt;, types: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;): property_map::PropertyMap<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun prepare_input(<br/>    keys: vector&lt;String&gt;,<br/>    types: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>): PropertyMap &#123;<br/>    let length &#61; vector::length(&amp;keys);<br/>    assert!(length &lt;&#61; MAX_PROPERTY_MAP_SIZE, error::invalid_argument(ETOO_MANY_PROPERTIES));<br/>    assert!(length &#61;&#61; vector::length(&amp;values), error::invalid_argument(EKEY_VALUE_COUNT_MISMATCH));<br/>    assert!(length &#61;&#61; vector::length(&amp;types), error::invalid_argument(EKEY_TYPE_COUNT_MISMATCH));<br/><br/>    let container &#61; simple_map::create&lt;String, PropertyValue&gt;();<br/>    while (!vector::is_empty(&amp;keys)) &#123;<br/>        let key &#61; vector::pop_back(&amp;mut keys);<br/>        assert!(<br/>            string::length(&amp;key) &lt;&#61; MAX_PROPERTY_NAME_LENGTH,<br/>            error::invalid_argument(EPROPERTY_MAP_KEY_TOO_LONG),<br/>        );<br/><br/>        let value &#61; vector::pop_back(&amp;mut values);<br/>        let type &#61; vector::pop_back(&amp;mut types);<br/><br/>        let new_type &#61; to_internal_type(type);<br/>        validate_type(new_type, value);<br/><br/>        simple_map::add(&amp;mut container, key, PropertyValue &#123; value, type: new_type &#125;);<br/>    &#125;;<br/><br/>    PropertyMap &#123; inner: container &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_to_external_type"></a>

## Function `to_external_type`

Maps <code>String</code> representation of types from their <code>u8</code> representation


<pre><code>fun to_external_type(type: u8): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun to_external_type(type: u8): String &#123;<br/>    if (type &#61;&#61; BOOL) &#123;<br/>        string::utf8(b&quot;bool&quot;)<br/>    &#125; else if (type &#61;&#61; U8) &#123;<br/>        string::utf8(b&quot;u8&quot;)<br/>    &#125; else if (type &#61;&#61; U16) &#123;<br/>        string::utf8(b&quot;u16&quot;)<br/>    &#125; else if (type &#61;&#61; U32) &#123;<br/>        string::utf8(b&quot;u32&quot;)<br/>    &#125; else if (type &#61;&#61; U64) &#123;<br/>        string::utf8(b&quot;u64&quot;)<br/>    &#125; else if (type &#61;&#61; U128) &#123;<br/>        string::utf8(b&quot;u128&quot;)<br/>    &#125; else if (type &#61;&#61; U256) &#123;<br/>        string::utf8(b&quot;u256&quot;)<br/>    &#125; else if (type &#61;&#61; ADDRESS) &#123;<br/>        string::utf8(b&quot;address&quot;)<br/>    &#125; else if (type &#61;&#61; BYTE_VECTOR) &#123;<br/>        string::utf8(b&quot;vector&lt;u8&gt;&quot;)<br/>    &#125; else if (type &#61;&#61; STRING) &#123;<br/>        string::utf8(b&quot;0x1::string::String&quot;)<br/>    &#125; else &#123;<br/>        abort (error::invalid_argument(ETYPE_INVALID))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_to_internal_type"></a>

## Function `to_internal_type`

Maps the <code>String</code> representation of types to <code>u8</code>


<pre><code>fun to_internal_type(type: string::String): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun to_internal_type(type: String): u8 &#123;<br/>    if (type &#61;&#61; string::utf8(b&quot;bool&quot;)) &#123;<br/>        BOOL<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u8&quot;)) &#123;<br/>        U8<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u16&quot;)) &#123;<br/>        U16<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u32&quot;)) &#123;<br/>        U32<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u64&quot;)) &#123;<br/>        U64<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u128&quot;)) &#123;<br/>        U128<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;u256&quot;)) &#123;<br/>        U256<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;address&quot;)) &#123;<br/>        ADDRESS<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;vector&lt;u8&gt;&quot;)) &#123;<br/>        BYTE_VECTOR<br/>    &#125; else if (type &#61;&#61; string::utf8(b&quot;0x1::string::String&quot;)) &#123;<br/>        STRING<br/>    &#125; else &#123;<br/>        abort (error::invalid_argument(ETYPE_INVALID))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_type_info_to_internal_type"></a>

## Function `type_info_to_internal_type`

Maps Move type to <code>u8</code> representation


<pre><code>fun type_info_to_internal_type&lt;T&gt;(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun type_info_to_internal_type&lt;T&gt;(): u8 &#123;<br/>    let type &#61; type_info::type_name&lt;T&gt;();<br/>    to_internal_type(type)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_validate_type"></a>

## Function `validate_type`

Validates property value type against its expected type


<pre><code>fun validate_type(type: u8, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun validate_type(type: u8, value: vector&lt;u8&gt;) &#123;<br/>    if (type &#61;&#61; BOOL) &#123;<br/>        from_bcs::to_bool(value);<br/>    &#125; else if (type &#61;&#61; U8) &#123;<br/>        from_bcs::to_u8(value);<br/>    &#125; else if (type &#61;&#61; U16) &#123;<br/>        from_bcs::to_u16(value);<br/>    &#125; else if (type &#61;&#61; U32) &#123;<br/>        from_bcs::to_u32(value);<br/>    &#125; else if (type &#61;&#61; U64) &#123;<br/>        from_bcs::to_u64(value);<br/>    &#125; else if (type &#61;&#61; U128) &#123;<br/>        from_bcs::to_u128(value);<br/>    &#125; else if (type &#61;&#61; U256) &#123;<br/>        from_bcs::to_u256(value);<br/>    &#125; else if (type &#61;&#61; ADDRESS) &#123;<br/>        from_bcs::to_address(value);<br/>    &#125; else if (type &#61;&#61; BYTE_VECTOR) &#123;<br/>        // nothing to validate...<br/>    &#125; else if (type &#61;&#61; STRING) &#123;<br/>        from_bcs::to_string(value);<br/>    &#125; else &#123;<br/>        abort (error::invalid_argument(ETYPE_MISMATCH))<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_generate_mutator_ref"></a>

## Function `generate_mutator_ref`



<pre><code>public fun generate_mutator_ref(ref: &amp;object::ConstructorRef): property_map::MutatorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mutator_ref(ref: &amp;ConstructorRef): MutatorRef &#123;<br/>    MutatorRef &#123; self: object::address_from_constructor_ref(ref) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_contains_key"></a>

## Function `contains_key`



<pre><code>public fun contains_key&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains_key&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): bool acquires PropertyMap &#123;<br/>    assert_exists(object::object_address(object));<br/>    let property_map &#61; borrow_global&lt;PropertyMap&gt;(object::object_address(object));<br/>    simple_map::contains_key(&amp;property_map.inner, key)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_length"></a>

## Function `length`



<pre><code>public fun length&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;T: key&gt;(object: &amp;Object&lt;T&gt;): u64 acquires PropertyMap &#123;<br/>    assert_exists(object::object_address(object));<br/>    let property_map &#61; borrow_global&lt;PropertyMap&gt;(object::object_address(object));<br/>    simple_map::length(&amp;property_map.inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read"></a>

## Function `read`

Read the property and get it&apos;s external type in it&apos;s bcs encoded format<br/><br/> The preferred method is to use <code>read_&lt;type&gt;</code> where the type is already known.


<pre><code>public fun read&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): (string::String, vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): (String, vector&lt;u8&gt;) acquires PropertyMap &#123;<br/>    assert_exists(object::object_address(object));<br/>    let property_map &#61; borrow_global&lt;PropertyMap&gt;(object::object_address(object));<br/>    let property_value &#61; simple_map::borrow(&amp;property_map.inner, key);<br/>    let new_type &#61; to_external_type(property_value.type);<br/>    (new_type, property_value.value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_assert_exists"></a>

## Function `assert_exists`



<pre><code>fun assert_exists(object: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_exists(object: address) &#123;<br/>    assert!(<br/>        exists&lt;PropertyMap&gt;(object),<br/>        error::not_found(EPROPERTY_MAP_DOES_NOT_EXIST),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_typed"></a>

## Function `read_typed`

Read a type and verify that the type is correct


<pre><code>fun read_typed&lt;T: key, V&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun read_typed&lt;T: key, V&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): vector&lt;u8&gt; acquires PropertyMap &#123;<br/>    let (type, value) &#61; read(object, key);<br/>    assert!(<br/>        type &#61;&#61; type_info::type_name&lt;V&gt;(),<br/>        error::invalid_argument(ETYPE_MISMATCH),<br/>    );<br/>    value<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_bool"></a>

## Function `read_bool`



<pre><code>public fun read_bool&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_bool&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): bool acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, bool&gt;(object, key);<br/>    from_bcs::to_bool(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u8"></a>

## Function `read_u8`



<pre><code>public fun read_u8&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u8&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u8 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u8&gt;(object, key);<br/>    from_bcs::to_u8(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u16"></a>

## Function `read_u16`



<pre><code>public fun read_u16&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u16<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u16&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u16 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u16&gt;(object, key);<br/>    from_bcs::to_u16(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u32"></a>

## Function `read_u32`



<pre><code>public fun read_u32&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u32&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u32 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u32&gt;(object, key);<br/>    from_bcs::to_u32(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u64"></a>

## Function `read_u64`



<pre><code>public fun read_u64&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u64&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u64 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u64&gt;(object, key);<br/>    from_bcs::to_u64(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u128"></a>

## Function `read_u128`



<pre><code>public fun read_u128&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u128&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u128 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u128&gt;(object, key);<br/>    from_bcs::to_u128(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_u256"></a>

## Function `read_u256`



<pre><code>public fun read_u256&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): u256<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_u256&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): u256 acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, u256&gt;(object, key);<br/>    from_bcs::to_u256(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_address"></a>

## Function `read_address`



<pre><code>public fun read_address&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_address&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): address acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, address&gt;(object, key);<br/>    from_bcs::to_address(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_bytes"></a>

## Function `read_bytes`



<pre><code>public fun read_bytes&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_bytes&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): vector&lt;u8&gt; acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, vector&lt;u8&gt;&gt;(object, key);<br/>    from_bcs::to_bytes(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_read_string"></a>

## Function `read_string`



<pre><code>public fun read_string&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;, key: &amp;string::String): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read_string&lt;T: key&gt;(object: &amp;Object&lt;T&gt;, key: &amp;String): String acquires PropertyMap &#123;<br/>    let value &#61; read_typed&lt;T, String&gt;(object, key);<br/>    from_bcs::to_string(value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_add"></a>

## Function `add`

Add a property, already bcs encoded as a <code>vector&lt;u8&gt;</code>


<pre><code>public fun add(ref: &amp;property_map::MutatorRef, key: string::String, type: string::String, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(ref: &amp;MutatorRef, key: String, type: String, value: vector&lt;u8&gt;) acquires PropertyMap &#123;<br/>    let new_type &#61; to_internal_type(type);<br/>    validate_type(new_type, value);<br/>    add_internal(ref, key, new_type, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_add_typed"></a>

## Function `add_typed`

Add a property that isn&apos;t already encoded as a <code>vector&lt;u8&gt;</code>


<pre><code>public fun add_typed&lt;T: drop&gt;(ref: &amp;property_map::MutatorRef, key: string::String, value: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_typed&lt;T: drop&gt;(ref: &amp;MutatorRef, key: String, value: T) acquires PropertyMap &#123;<br/>    let type &#61; type_info_to_internal_type&lt;T&gt;();<br/>    add_internal(ref, key, type, bcs::to_bytes(&amp;value));<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_add_internal"></a>

## Function `add_internal`



<pre><code>fun add_internal(ref: &amp;property_map::MutatorRef, key: string::String, type: u8, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun add_internal(ref: &amp;MutatorRef, key: String, type: u8, value: vector&lt;u8&gt;) acquires PropertyMap &#123;<br/>    assert_exists(ref.self);<br/>    let property_map &#61; borrow_global_mut&lt;PropertyMap&gt;(ref.self);<br/>    simple_map::add(&amp;mut property_map.inner, key, PropertyValue &#123; type, value &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_update"></a>

## Function `update`

Updates a property in place already bcs encoded


<pre><code>public fun update(ref: &amp;property_map::MutatorRef, key: &amp;string::String, type: string::String, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update(ref: &amp;MutatorRef, key: &amp;String, type: String, value: vector&lt;u8&gt;) acquires PropertyMap &#123;<br/>    let new_type &#61; to_internal_type(type);<br/>    validate_type(new_type, value);<br/>    update_internal(ref, key, new_type, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_update_typed"></a>

## Function `update_typed`

Updates a property in place that is not already bcs encoded


<pre><code>public fun update_typed&lt;T: drop&gt;(ref: &amp;property_map::MutatorRef, key: &amp;string::String, value: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_typed&lt;T: drop&gt;(ref: &amp;MutatorRef, key: &amp;String, value: T) acquires PropertyMap &#123;<br/>    let type &#61; type_info_to_internal_type&lt;T&gt;();<br/>    update_internal(ref, key, type, bcs::to_bytes(&amp;value));<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_update_internal"></a>

## Function `update_internal`



<pre><code>fun update_internal(ref: &amp;property_map::MutatorRef, key: &amp;string::String, type: u8, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun update_internal(ref: &amp;MutatorRef, key: &amp;String, type: u8, value: vector&lt;u8&gt;) acquires PropertyMap &#123;<br/>    assert_exists(ref.self);<br/>    let property_map &#61; borrow_global_mut&lt;PropertyMap&gt;(ref.self);<br/>    let old_value &#61; simple_map::borrow_mut(&amp;mut property_map.inner, key);<br/>    &#42;old_value &#61; PropertyValue &#123; type, value &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_remove"></a>

## Function `remove`

Removes a property from the map, ensuring that it does in fact exist


<pre><code>public fun remove(ref: &amp;property_map::MutatorRef, key: &amp;string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove(ref: &amp;MutatorRef, key: &amp;String) acquires PropertyMap &#123;<br/>    assert_exists(ref.self);<br/>    let property_map &#61; borrow_global_mut&lt;PropertyMap&gt;(ref.self);<br/>    simple_map::remove(&amp;mut property_map.inner, key);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_property_map_assert_end_to_end_input"></a>

## Function `assert_end_to_end_input`



<pre><code>fun assert_end_to_end_input(object: object::Object&lt;object::ObjectCore&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_end_to_end_input(object: Object&lt;ObjectCore&gt;) acquires PropertyMap &#123;<br/>    assert!(read_bool(&amp;object, &amp;string::utf8(b&quot;bool&quot;)), 0);<br/>    assert!(read_u8(&amp;object, &amp;string::utf8(b&quot;u8&quot;)) &#61;&#61; 0x12, 1);<br/>    assert!(read_u16(&amp;object, &amp;string::utf8(b&quot;u16&quot;)) &#61;&#61; 0x1234, 2);<br/>    assert!(read_u32(&amp;object, &amp;string::utf8(b&quot;u32&quot;)) &#61;&#61; 0x12345678, 3);<br/>    assert!(read_u64(&amp;object, &amp;string::utf8(b&quot;u64&quot;)) &#61;&#61; 0x1234567812345678, 4);<br/>    assert!(read_u128(&amp;object, &amp;string::utf8(b&quot;u128&quot;)) &#61;&#61; 0x12345678123456781234567812345678, 5);<br/>    assert!(<br/>        read_u256(<br/>            &amp;object,<br/>            &amp;string::utf8(b&quot;u256&quot;)<br/>        ) &#61;&#61; 0x1234567812345678123456781234567812345678123456781234567812345678,<br/>        6<br/>    );<br/>    assert!(read_bytes(&amp;object, &amp;string::utf8(b&quot;vector&lt;u8&gt;&quot;)) &#61;&#61; vector[0x01], 7);<br/>    assert!(read_string(&amp;object, &amp;string::utf8(b&quot;0x1::string::String&quot;)) &#61;&#61; string::utf8(b&quot;a&quot;), 8);<br/><br/>    assert!(length(&amp;object) &#61;&#61; 9, 9);<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
