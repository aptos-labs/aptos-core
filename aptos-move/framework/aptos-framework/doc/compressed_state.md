
<a id="0x1_compressed_state"></a>

# Module `0x1::compressed_state`



-  [Struct `CompressedCore`](#0x1_compressed_state_CompressedCore)
-  [Struct `TypedValue`](#0x1_compressed_state_TypedValue)
-  [Resource `CompressedState`](#0x1_compressed_state_CompressedState)
-  [Struct `Compress`](#0x1_compressed_state_Compress)
-  [Constants](#@Constants_0)
-  [Function `enable_compression_for_custom_core`](#0x1_compressed_state_enable_compression_for_custom_core)
-  [Function `compress`](#0x1_compressed_state_compress)
-  [Function `get_hash`](#0x1_compressed_state_get_hash)
-  [Function `get_onchain_data`](#0x1_compressed_state_get_onchain_data)
-  [Function `get`](#0x1_compressed_state_get)
-  [Function `decompress_and_remove`](#0x1_compressed_state_decompress_and_remove)
-  [Function `deserialize_value`](#0x1_compressed_state_deserialize_value)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="unique_key.md#0x1_unique_key">0x1::unique_key</a>;
<b>use</b> <a href="util.md#0x1_util">0x1::util</a>;
</code></pre>



<a id="0x1_compressed_state_CompressedCore"></a>

## Struct `CompressedCore`



<pre><code><b>struct</b> <a href="compressed_state.md#0x1_compressed_state_CompressedCore">CompressedCore</a>&lt;T: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>on_chain_core: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_compressed_state_TypedValue"></a>

## Struct `TypedValue`

Not using Any, to not require unnecessary double serialization.


<pre><code><b>struct</b> <a href="compressed_state.md#0x1_compressed_state_TypedValue">TypedValue</a>&lt;V: drop, store&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>value: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_compressed_state_CompressedState"></a>

## Resource `CompressedState`



<pre><code><b>struct</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;u64, <a href="compressed_state.md#0x1_compressed_state_CompressedCore">compressed_state::CompressedCore</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_compressed_state_Compress"></a>

## Struct `Compress`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="compressed_state.md#0x1_compressed_state_Compress">Compress</a>&lt;T: <b>copy</b>, drop, store, V: drop, store&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>core: <a href="compressed_state.md#0x1_compressed_state_CompressedCore">compressed_state::CompressedCore</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>typed_value: <a href="compressed_state.md#0x1_compressed_state_TypedValue">compressed_state::TypedValue</a>&lt;V&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_compressed_state_ETYPE_MISMATCH"></a>



<pre><code><b>const</b> <a href="compressed_state.md#0x1_compressed_state_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x1_compressed_state_ECOMPRESSED_ID_ALREADY_PRESENT"></a>

compressed_id already exists


<pre><code><b>const</b> <a href="compressed_state.md#0x1_compressed_state_ECOMPRESSED_ID_ALREADY_PRESENT">ECOMPRESSED_ID_ALREADY_PRESENT</a>: u64 = 2;
</code></pre>



<a id="0x1_compressed_state_ECORE_COMPRESSION_ALREADY_REGISTERED"></a>



<pre><code><b>const</b> <a href="compressed_state.md#0x1_compressed_state_ECORE_COMPRESSION_ALREADY_REGISTERED">ECORE_COMPRESSION_ALREADY_REGISTERED</a>: u64 = 3;
</code></pre>



<a id="0x1_compressed_state_EHASH_DOESNT_MATCH"></a>



<pre><code><b>const</b> <a href="compressed_state.md#0x1_compressed_state_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_compressed_state_enable_compression_for_custom_core"></a>

## Function `enable_compression_for_custom_core`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_enable_compression_for_custom_core">enable_compression_for_custom_core</a>&lt;T: <b>copy</b>, drop, store&gt;(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_enable_compression_for_custom_core">enable_compression_for_custom_core</a>&lt;T: store + drop + <b>copy</b>&gt;(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt; {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    };
    <b>assert</b>!(!<b>exists</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework_signer)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="compressed_state.md#0x1_compressed_state_ECORE_COMPRESSION_ALREADY_REGISTERED">ECORE_COMPRESSION_ALREADY_REGISTERED</a>));
    <b>move_to</b>(framework_signer, <a href="compressed_state.md#0x1_compressed_state">compressed_state</a>);
}
</code></pre>



</details>

<a id="0x1_compressed_state_compress"></a>

## Function `compress`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_compress">compress</a>&lt;T: <b>copy</b>, drop, store, V: drop, store&gt;(on_chain_core: T, value: V): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_compress">compress</a>&lt;T: store + drop + <b>copy</b>, V: drop + store&gt;(on_chain_core: T, value: V): u64 <b>acquires</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a> {
    <b>let</b> typed_value = <a href="compressed_state.md#0x1_compressed_state_TypedValue">TypedValue</a>&lt;V&gt; {
        type_name: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;(),
        value,
    };
    <b>let</b> compressed_id = <a href="unique_key.md#0x1_unique_key_generate_unique_key">unique_key::generate_unique_key</a>(@aptos_framework);
    // TODO create delayed <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> -&gt; <b>address</b>, <b>to</b> support aggregators.
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&typed_value)));

    <b>let</b> core = <a href="compressed_state.md#0x1_compressed_state_CompressedCore">CompressedCore</a> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        on_chain_core,
    };
    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="compressed_state.md#0x1_compressed_state_Compress">Compress</a>&lt;T, V&gt; {
        compressed_id,
        core: <b>copy</b> core,
        typed_value,
    };
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="event.md#0x1_event">event</a>);

    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <b>borrow_global_mut</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(@aptos_framework);

    <b>assert</b>!(!<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_contains">smart_table::contains</a>(&<a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="compressed_state.md#0x1_compressed_state_ECOMPRESSED_ID_ALREADY_PRESENT">ECOMPRESSED_ID_ALREADY_PRESENT</a>));

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(&<b>mut</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id, core);
    compressed_id
}
</code></pre>



</details>

<a id="0x1_compressed_state_get_hash"></a>

## Function `get_hash`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get_hash">get_hash</a>&lt;T: <b>copy</b>, drop, store&gt;(compressed_id: u64): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get_hash">get_hash</a>&lt;T: store + drop + <b>copy</b>&gt;(compressed_id: u64): <b>address</b> <b>acquires</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a> {
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <b>borrow_global</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(@aptos_framework);

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id).<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
}
</code></pre>



</details>

<a id="0x1_compressed_state_get_onchain_data"></a>

## Function `get_onchain_data`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get_onchain_data">get_onchain_data</a>&lt;T: <b>copy</b>, drop, store&gt;(compressed_id: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get_onchain_data">get_onchain_data</a>&lt;T: store + drop + <b>copy</b>&gt;(compressed_id: u64): T <b>acquires</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a> {
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <b>borrow_global</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(@aptos_framework);

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id).on_chain_core
}
</code></pre>



</details>

<a id="0x1_compressed_state_get"></a>

## Function `get`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get">get</a>&lt;T: <b>copy</b>, drop, store, V: <b>copy</b>, drop, store&gt;(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (T, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_get">get</a>&lt;T: store + drop + <b>copy</b>, V: drop + store + <b>copy</b>&gt;(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (T, V) <b>acquires</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a> {
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <b>borrow_global</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(@aptos_framework);
    <b>let</b> core = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow">smart_table::borrow</a>(&<a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id);

    <b>let</b> value = <a href="compressed_state.md#0x1_compressed_state_deserialize_value">deserialize_value</a>&lt;V&gt;(core.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, serialized);
    (core.on_chain_core, value)
}
</code></pre>



</details>

<a id="0x1_compressed_state_decompress_and_remove"></a>

## Function `decompress_and_remove`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_decompress_and_remove">decompress_and_remove</a>&lt;T: <b>copy</b>, drop, store, V: drop, store&gt;(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (T, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_state.md#0x1_compressed_state_decompress_and_remove">decompress_and_remove</a>&lt;T: store + drop + <b>copy</b>, V: drop + store&gt;(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (T, V) <b>acquires</b> <a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a> {
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a> = <b>borrow_global_mut</b>&lt;<a href="compressed_state.md#0x1_compressed_state_CompressedState">CompressedState</a>&lt;T&gt;&gt;(@aptos_framework);
    <b>let</b> core = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(&<b>mut</b> <a href="compressed_state.md#0x1_compressed_state">compressed_state</a>.<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, compressed_id);

    <b>let</b> value = <a href="compressed_state.md#0x1_compressed_state_deserialize_value">deserialize_value</a>&lt;V&gt;(core.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, serialized);
    (core.on_chain_core, value)
}
</code></pre>



</details>

<a id="0x1_compressed_state_deserialize_value"></a>

## Function `deserialize_value`



<pre><code><b>fun</b> <a href="compressed_state.md#0x1_compressed_state_deserialize_value">deserialize_value</a>&lt;V: drop, store&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="compressed_state.md#0x1_compressed_state_deserialize_value">deserialize_value</a>&lt;V: drop + store&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <b>address</b>, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): V {
    <b>let</b> data_hash = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(serialized));

    <b>assert</b>!(data_hash == <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="compressed_state.md#0x1_compressed_state_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>));
    <b>let</b> <a href="compressed_state.md#0x1_compressed_state_TypedValue">TypedValue</a> {
        value,
        type_name,
    } = <a href="util.md#0x1_util_from_bytes">util::from_bytes</a>&lt;<a href="compressed_state.md#0x1_compressed_state_TypedValue">TypedValue</a>&lt;V&gt;&gt;(serialized);
    // TODO is deserialization from wrong type, and then checking for correct type and aborting safe?
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;V&gt;() == type_name, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="compressed_state.md#0x1_compressed_state_ETYPE_MISMATCH">ETYPE_MISMATCH</a>));
    value
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
