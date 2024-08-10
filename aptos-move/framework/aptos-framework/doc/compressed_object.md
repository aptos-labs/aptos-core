
<a id="0x1_compressed_object"></a>

# Module `0x1::compressed_object`



-  [Struct `CompressedObjectCore`](#0x1_compressed_object_CompressedObjectCore)
-  [Struct `CompressedObject`](#0x1_compressed_object_CompressedObject)
-  [Struct `DecompressingObject`](#0x1_compressed_object_DecompressingObject)
-  [Constants](#@Constants_0)
-  [Function `initialize_compressed_object`](#0x1_compressed_object_initialize_compressed_object)
-  [Function `compress_existing_object`](#0x1_compressed_object_compress_existing_object)
-  [Function `decompress_object`](#0x1_compressed_object_decompress_object)
-  [Function `finish_decompressing`](#0x1_compressed_object_finish_decompressing)


<pre><code><b>use</b> <a href="compressed_state.md#0x1_compressed_state">0x1::compressed_state</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/copyable_any_map.md#0x1_copyable_any_map">0x1::copyable_any_map</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
</code></pre>



<a id="0x1_compressed_object_CompressedObjectCore"></a>

## Struct `CompressedObjectCore`



<pre><code><b>struct</b> <a href="compressed_object.md#0x1_compressed_object_CompressedObjectCore">CompressedObjectCore</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_compressed_object_CompressedObject"></a>

## Struct `CompressedObject`



<pre><code><b>struct</b> <a href="compressed_object.md#0x1_compressed_object_CompressedObject">CompressedObject</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_CreateAtAddressRef">object::CreateAtAddressRef</a></code>
</dt>
<dd>
 Object address used when object is uncompressed
</dd>
<dt>
<code>resources: <a href="../../aptos-stdlib/doc/copyable_any_map.md#0x1_copyable_any_map_AnyMap">copyable_any_map::AnyMap</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_compressed_object_DecompressingObject"></a>

## Struct `DecompressingObject`



<pre><code><b>struct</b> <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">DecompressingObject</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>resources: <a href="../../aptos-stdlib/doc/copyable_any_map.md#0x1_copyable_any_map_AnyMap">copyable_any_map::AnyMap</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_compressed_object_EDECOMPRESSION_NOT_FINISHED"></a>



<pre><code><b>const</b> <a href="compressed_object.md#0x1_compressed_object_EDECOMPRESSION_NOT_FINISHED">EDECOMPRESSION_NOT_FINISHED</a>: u64 = 1;
</code></pre>



<a id="0x1_compressed_object_initialize_compressed_object"></a>

## Function `initialize_compressed_object`



<pre><code>entry <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_initialize_compressed_object">initialize_compressed_object</a>(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_initialize_compressed_object">initialize_compressed_object</a>(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="compressed_state.md#0x1_compressed_state_enable_compression_for_custom_core">compressed_state::enable_compression_for_custom_core</a>&lt;<a href="compressed_object.md#0x1_compressed_object_CompressedObjectCore">CompressedObjectCore</a>&gt;(framework_signer);
}
</code></pre>



</details>

<a id="0x1_compressed_object_compress_existing_object"></a>

## Function `compress_existing_object`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_compress_existing_object">compress_existing_object</a>(ref: <a href="object.md#0x1_object_DeleteAndRecreateRef">object::DeleteAndRecreateRef</a>, resources: <a href="../../aptos-stdlib/doc/copyable_any_map.md#0x1_copyable_any_map_AnyMap">copyable_any_map::AnyMap</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_compress_existing_object">compress_existing_object</a>(ref: DeleteAndRecreateRef, resources: AnyMap) {
    <b>let</b> (<a href="object.md#0x1_object">object</a>, owner, allow_ungated_transfer) = <a href="object.md#0x1_object_delete_and_can_recreate">object::delete_and_can_recreate</a>(ref);

    <b>let</b> compressed_core = <a href="compressed_object.md#0x1_compressed_object_CompressedObjectCore">CompressedObjectCore</a> {
        owner,
        allow_ungated_transfer,
    };

    <b>let</b> <a href="compressed_object.md#0x1_compressed_object">compressed_object</a> = <a href="compressed_object.md#0x1_compressed_object_CompressedObject">CompressedObject</a> {
        <a href="object.md#0x1_object">object</a>,
        resources,
    };

    <a href="compressed_state.md#0x1_compressed_state_compress">compressed_state::compress</a>(compressed_core, <a href="compressed_object.md#0x1_compressed_object">compressed_object</a>);
}
</code></pre>



</details>

<a id="0x1_compressed_object_decompress_object"></a>

## Function `decompress_object`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_decompress_object">decompress_object</a>(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">compressed_object::DecompressingObject</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_decompress_object">decompress_object</a>(compressed_id: u64, serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (ConstructorRef, <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">DecompressingObject</a>) {
    <b>let</b> (
        <a href="compressed_object.md#0x1_compressed_object_CompressedObjectCore">CompressedObjectCore</a> {
            owner,
            allow_ungated_transfer: _,
        },
        <a href="compressed_object.md#0x1_compressed_object_CompressedObject">CompressedObject</a> {
            <a href="object.md#0x1_object">object</a>,
            resources,
        }
    ) = <a href="compressed_state.md#0x1_compressed_state_decompress_and_remove">compressed_state::decompress_and_remove</a>&lt;<a href="compressed_object.md#0x1_compressed_object_CompressedObjectCore">CompressedObjectCore</a>, <a href="compressed_object.md#0x1_compressed_object_CompressedObject">CompressedObject</a>&gt;(compressed_id, serialized);

    <b>let</b> constructor_ref = <a href="object.md#0x1_object_create_object_at_address_from_ref">object::create_object_at_address_from_ref</a>(owner, <a href="object.md#0x1_object">object</a>);

    (constructor_ref, <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">DecompressingObject</a> {
        resources: resources,
    })
}
</code></pre>



</details>

<a id="0x1_compressed_object_finish_decompressing"></a>

## Function `finish_decompressing`



<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_finish_decompressing">finish_decompressing</a>(<a href="object.md#0x1_object">object</a>: <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">compressed_object::DecompressingObject</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="compressed_object.md#0x1_compressed_object_finish_decompressing">finish_decompressing</a>(<a href="object.md#0x1_object">object</a>: <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">DecompressingObject</a>) {
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/copyable_any_map.md#0x1_copyable_any_map_length">copyable_any_map::length</a>(&<a href="object.md#0x1_object">object</a>.resources) == 0, <a href="compressed_object.md#0x1_compressed_object_EDECOMPRESSION_NOT_FINISHED">EDECOMPRESSION_NOT_FINISHED</a>);
    <b>let</b> <a href="compressed_object.md#0x1_compressed_object_DecompressingObject">DecompressingObject</a> {
        resources: _
    } = <a href="object.md#0x1_object">object</a>;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
