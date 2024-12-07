
<a id="0x1_external_unique_state"></a>

# Module `0x1::external_unique_state`

Onchain resource containing a set of ExternalValues.
Set is without duplicates - i.e. same value cannot be stored twice.

Replaces <code><b>move_to</b></code> / <code><b>borrow_global_mut</b></code> / <code><b>move_from</b></code>
with <code>move_to_external_storage</code> / <code>borrow_mut</code> / <code>move_from_external_storage</code>

Provides equivalent access restrictions as <code><b>move_to</b></code> / <code><b>borrow_global_mut</b></code> / <code><b>move_from</b></code>
byte instructions have (which are that only declaring module can call them),
via each type having a corresponding witness that needs to be provided.
(if the module doesn't give out the witness)


-  [Resource `ExternalUniqueState`](#0x1_external_unique_state_ExternalUniqueState)
-  [Struct `MutableHandle`](#0x1_external_unique_state_MutableHandle)
-  [Constants](#@Constants_0)
-  [Function `enable_external_storage_for_type`](#0x1_external_unique_state_enable_external_storage_for_type)
-  [Function `move_to_external_storage`](#0x1_external_unique_state_move_to_external_storage)
-  [Function `get_copy`](#0x1_external_unique_state_get_copy)
-  [Function `move_from_external_storage`](#0x1_external_unique_state_move_from_external_storage)
-  [Function `borrow_mut`](#0x1_external_unique_state_borrow_mut)
-  [Function `handle_get_mut_value`](#0x1_external_unique_state_handle_get_mut_value)
-  [Function `handle_store`](#0x1_external_unique_state_handle_store)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="verified_external_value.md#0x1_verified_external_value">0x1::verified_external_value</a>;
</code></pre>



<a id="0x1_external_unique_state_ExternalUniqueState"></a>

## Resource `ExternalUniqueState`

Resource containing all ExternalValues of a given type.
It also keeps a witness, which is required to be provided in order to access values of this type.


<pre><code><b>struct</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T: drop, store, V: store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>witness: V</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_external_unique_state_MutableHandle"></a>

## Struct `MutableHandle`

A handle containing the value, which can be mutated, and then stored back.


<pre><code><b>struct</b> <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a>&lt;T: drop, store&gt;
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>typed_value: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_external_unique_state_ETYPE_MISMATCH"></a>



<pre><code><b>const</b> <a href="external_unique_state.md#0x1_external_unique_state_ETYPE_MISMATCH">ETYPE_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x1_external_unique_state_EHASH_DOESNT_MATCH"></a>



<pre><code><b>const</b> <a href="external_unique_state.md#0x1_external_unique_state_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_external_unique_state_ECOMPRESSED_ID_ALREADY_PRESENT"></a>

compressed_id already exists


<pre><code><b>const</b> <a href="external_unique_state.md#0x1_external_unique_state_ECOMPRESSED_ID_ALREADY_PRESENT">ECOMPRESSED_ID_ALREADY_PRESENT</a>: u64 = 2;
</code></pre>



<a id="0x1_external_unique_state_ECORE_COMPRESSION_ALREADY_REGISTERED"></a>



<pre><code><b>const</b> <a href="external_unique_state.md#0x1_external_unique_state_ECORE_COMPRESSION_ALREADY_REGISTERED">ECORE_COMPRESSION_ALREADY_REGISTERED</a>: u64 = 3;
</code></pre>



<a id="0x1_external_unique_state_EWITNESS_MISMATCH"></a>



<pre><code><b>const</b> <a href="external_unique_state.md#0x1_external_unique_state_EWITNESS_MISMATCH">EWITNESS_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x1_external_unique_state_enable_external_storage_for_type"></a>

## Function `enable_external_storage_for_type`

Registers a particular type to be able to be stored in external state, with access guarded by the provided witness.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_enable_external_storage_for_type">enable_external_storage_for_type</a>&lt;T: drop, store, V: store&gt;(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, witness: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_enable_external_storage_for_type">enable_external_storage_for_type</a>&lt;T: drop + store, V: store&gt;(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, witness: V) {
    <b>let</b> compressed_state = <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T, V&gt; {
        witness: witness,
        values: <a href="verified_external_value.md#0x1_verified_external_value_new_set">verified_external_value::new_set</a>()
    };
    <b>assert</b>!(!<b>exists</b>&lt;<a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T, V&gt;&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework_signer)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="external_unique_state.md#0x1_external_unique_state_ECORE_COMPRESSION_ALREADY_REGISTERED">ECORE_COMPRESSION_ALREADY_REGISTERED</a>));
    <b>move_to</b>(framework_signer, compressed_state);
}
</code></pre>



</details>

<a id="0x1_external_unique_state_move_to_external_storage"></a>

## Function `move_to_external_storage`



<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_move_to_external_storage">move_to_external_storage</a>&lt;T: drop, store, V: store&gt;(value: T, witness: &V): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_move_to_external_storage">move_to_external_storage</a>&lt;T: drop + store, V: store&gt;(value: T, witness: &V): u256 <b>acquires</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a> {
    <b>let</b> external_state = <b>borrow_global_mut</b>&lt;<a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T, V&gt;&gt;(@aptos_framework);
    <b>assert</b>!(witness == &external_state.witness, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="external_unique_state.md#0x1_external_unique_state_EWITNESS_MISMATCH">EWITNESS_MISMATCH</a>));

    <b>let</b> external_value = <a href="verified_external_value.md#0x1_verified_external_value_move_to_external_storage">verified_external_value::move_to_external_storage</a>(value);
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = external_value.get_hash();
    external_state.values.add(external_value);
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
}
</code></pre>



</details>

<a id="0x1_external_unique_state_get_copy"></a>

## Function `get_copy`



<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_get_copy">get_copy</a>&lt;T: <b>copy</b>, drop, store, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_get_copy">get_copy</a>&lt;T: store + drop + <b>copy</b>, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): T <b>acquires</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a> {
    <b>let</b> external_state = <b>borrow_global</b>&lt;<a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T, V&gt;&gt;(@aptos_framework);
    <b>assert</b>!(witness == &external_state.witness, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="external_unique_state.md#0x1_external_unique_state_EWITNESS_MISMATCH">EWITNESS_MISMATCH</a>));
    external_state.values.<a href="external_unique_state.md#0x1_external_unique_state_get_copy">get_copy</a>(<a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">verified_external_value::bytes_to_hash</a>(external_bytes)).into_value(external_bytes)
}
</code></pre>



</details>

<a id="0x1_external_unique_state_move_from_external_storage"></a>

## Function `move_from_external_storage`



<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_move_from_external_storage">move_from_external_storage</a>&lt;T: drop, store, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_move_from_external_storage">move_from_external_storage</a>&lt;T: drop + store, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): T <b>acquires</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a> {
    <b>let</b> external_state = <b>borrow_global_mut</b>&lt;<a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a>&lt;T, V&gt;&gt;(@aptos_framework);
    <b>assert</b>!(witness == &external_state.witness, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="external_unique_state.md#0x1_external_unique_state_EWITNESS_MISMATCH">EWITNESS_MISMATCH</a>));
    external_state.values.remove(<a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">verified_external_value::bytes_to_hash</a>(external_bytes)).into_value(external_bytes)
}
</code></pre>



</details>

<a id="0x1_external_unique_state_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_borrow_mut">borrow_mut</a>&lt;T: drop, store, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">external_unique_state::MutableHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_borrow_mut">borrow_mut</a>&lt;T: drop + store, V: store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, witness: &V): <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a>&lt;T&gt; <b>acquires</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a> {
    <b>let</b> typed_value = <a href="external_unique_state.md#0x1_external_unique_state_move_from_external_storage">move_from_external_storage</a>&lt;T, V&gt;(external_bytes, witness);
    <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a> {
        typed_value,
    }
}
</code></pre>



</details>

<a id="0x1_external_unique_state_handle_get_mut_value"></a>

## Function `handle_get_mut_value`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_handle_get_mut_value">handle_get_mut_value</a>&lt;T: drop, store&gt;(self: &<b>mut</b> <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">external_unique_state::MutableHandle</a>&lt;T&gt;): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_handle_get_mut_value">handle_get_mut_value</a>&lt;T: drop + store&gt;(self: &<b>mut</b> <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a>&lt;T&gt;): &<b>mut</b> T {
    &<b>mut</b> self.typed_value
}
</code></pre>



</details>

<a id="0x1_external_unique_state_handle_store"></a>

## Function `handle_store`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_handle_store">handle_store</a>&lt;T: drop, store, V: store&gt;(self: <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">external_unique_state::MutableHandle</a>&lt;T&gt;, witness: &V): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="external_unique_state.md#0x1_external_unique_state_handle_store">handle_store</a>&lt;T: drop + store, V: store&gt;(self: <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a>&lt;T&gt;, witness: &V): u256 <b>acquires</b> <a href="external_unique_state.md#0x1_external_unique_state_ExternalUniqueState">ExternalUniqueState</a> {
    <b>let</b> <a href="external_unique_state.md#0x1_external_unique_state_MutableHandle">MutableHandle</a> {
        typed_value,
    } = self;
    <a href="external_unique_state.md#0x1_external_unique_state_move_to_external_storage">move_to_external_storage</a>(typed_value, witness)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
