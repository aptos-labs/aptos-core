
<a id="0x1_external_object"></a>

# Module `0x1::external_object`

Allowing objects (ObjectCore, together with all resources attached to it) to be stored
in external storage, with keeping only the hash onchain. That allows us to retrieve it later._

Pair of functions <code>move_existing_object_to_external_storage</code> and <code>move_external_object_to_state</code>
allow any deletable object to be moved to external storage, and back to onchain state.


-  [Struct `ExternalObjectWitness`](#0x1_external_object_ExternalObjectWitness)
-  [Struct `ExternalObject`](#0x1_external_object_ExternalObject)
-  [Struct `MovingToStateObject`](#0x1_external_object_MovingToStateObject)
-  [Struct `ObjectMovedToExternalStorage`](#0x1_external_object_ObjectMovedToExternalStorage)
-  [Constants](#@Constants_0)
-  [Function `initialize_external_object`](#0x1_external_object_initialize_external_object)
-  [Function `move_existing_object_to_external_storage`](#0x1_external_object_move_existing_object_to_external_storage)
-  [Function `move_external_object_to_state`](#0x1_external_object_move_external_object_to_state)
-  [Function `get_resources_mut`](#0x1_external_object_get_resources_mut)
-  [Function `destroy_empty`](#0x1_external_object_destroy_empty)
-  [Function `transfer`](#0x1_external_object_transfer)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map">0x1::any_map</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="external_unique_state.md#0x1_external_unique_state">0x1::external_unique_state</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
</code></pre>



<a id="0x1_external_object_ExternalObjectWitness"></a>

## Struct `ExternalObjectWitness`



<pre><code><b>struct</b> <a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_external_object_ExternalObject"></a>

## Struct `ExternalObject`



<pre><code><b>struct</b> <a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_ref: <a href="object.md#0x1_object_CreateAtAddressRef">object::CreateAtAddressRef</a></code>
</dt>
<dd>
 Object address used when object is uncompressed
</dd>
<dt>
<code>resources: <a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mut_permission: <a href="../../aptos-stdlib/doc/any.md#0x1_any_Any">any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_external_object_MovingToStateObject"></a>

## Struct `MovingToStateObject`

Undropable value, which makes sure whole object was consumed,
when moving object from external storage to onchain state.


<pre><code><b>struct</b> <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>resources: <a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_external_object_ObjectMovedToExternalStorage"></a>

## Struct `ObjectMovedToExternalStorage`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="external_object.md#0x1_external_object_ObjectMovedToExternalStorage">ObjectMovedToExternalStorage</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_external_object_EMOVING_TO_STATE_NOT_FINISHED"></a>



<pre><code><b>const</b> <a href="external_object.md#0x1_external_object_EMOVING_TO_STATE_NOT_FINISHED">EMOVING_TO_STATE_NOT_FINISHED</a>: u64 = 1;
</code></pre>



<a id="0x1_external_object_EPERMISSION_DOESNT_MATCH"></a>



<pre><code><b>const</b> <a href="external_object.md#0x1_external_object_EPERMISSION_DOESNT_MATCH">EPERMISSION_DOESNT_MATCH</a>: u64 = 2;
</code></pre>



<a id="0x1_external_object_initialize_external_object"></a>

## Function `initialize_external_object`



<pre><code>entry <b>fun</b> <a href="external_object.md#0x1_external_object_initialize_external_object">initialize_external_object</a>(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="external_object.md#0x1_external_object_initialize_external_object">initialize_external_object</a>(framework_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="external_unique_state.md#0x1_external_unique_state_enable_external_storage_for_type">external_unique_state::enable_external_storage_for_type</a>&lt;<a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a>, <a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a>&gt;(framework_signer, <a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> {});
}
</code></pre>



</details>

<a id="0x1_external_object_move_existing_object_to_external_storage"></a>

## Function `move_existing_object_to_external_storage`



<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_move_existing_object_to_external_storage">move_existing_object_to_external_storage</a>&lt;P: drop, store&gt;(ref: <a href="object.md#0x1_object_DeleteAndRecreateRef">object::DeleteAndRecreateRef</a>, resources: <a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>, mut_permission: P)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_move_existing_object_to_external_storage">move_existing_object_to_external_storage</a>&lt;P: drop + store&gt;(ref: DeleteAndRecreateRef, resources: AnyMap, mut_permission: P) {
    <b>let</b> object_addr = ref.address_from_delete_and_recreate_ref();
    <b>let</b> object_ref = <a href="object.md#0x1_object_delete_and_can_recreate">object::delete_and_can_recreate</a>(ref);

    <b>let</b> compressed_object = <a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a> {
        object_ref,
        resources,
        mut_permission: <a href="../../aptos-stdlib/doc/any.md#0x1_any_pack">any::pack</a>(mut_permission),
    };

    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = <a href="external_unique_state.md#0x1_external_unique_state_move_to_external_storage">external_unique_state::move_to_external_storage</a>(compressed_object, &<a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> {});

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="external_object.md#0x1_external_object_ObjectMovedToExternalStorage">ObjectMovedToExternalStorage</a> {
        object_addr,
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
    });
}
</code></pre>



</details>

<a id="0x1_external_object_move_external_object_to_state"></a>

## Function `move_external_object_to_state`



<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_move_external_object_to_state">move_external_object_to_state</a>&lt;P: drop, store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, mut_permission: P): (<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, <a href="external_object.md#0x1_external_object_MovingToStateObject">external_object::MovingToStateObject</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_move_external_object_to_state">move_external_object_to_state</a>&lt;P: drop + store&gt;(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, mut_permission: P): (ConstructorRef, <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a>) {
    <b>let</b> <a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a> {
        object_ref,
        resources,
        mut_permission: external_mut_perm,
    } = <a href="external_unique_state.md#0x1_external_unique_state_move_from_external_storage">external_unique_state::move_from_external_storage</a>&lt;<a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a>, <a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a>&gt;(external_bytes, &<a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> {});
    <b>assert</b>!(mut_permission == external_mut_perm.unpack(), <a href="external_object.md#0x1_external_object_EPERMISSION_DOESNT_MATCH">EPERMISSION_DOESNT_MATCH</a>);

    <b>let</b> constructor_ref = <a href="object.md#0x1_object_create_object_at_address_from_ref">object::create_object_at_address_from_ref</a>(object_ref);

    (constructor_ref, <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a> {
        resources: resources,
    })
}
</code></pre>



</details>

<a id="0x1_external_object_get_resources_mut"></a>

## Function `get_resources_mut`



<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_get_resources_mut">get_resources_mut</a>(self: &<b>mut</b> <a href="external_object.md#0x1_external_object_MovingToStateObject">external_object::MovingToStateObject</a>): &<b>mut</b> <a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map_AnyMap">any_map::AnyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_get_resources_mut">get_resources_mut</a>(self: &<b>mut</b> <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a>): &<b>mut</b> AnyMap {
    &<b>mut</b> self.resources
}
</code></pre>



</details>

<a id="0x1_external_object_destroy_empty"></a>

## Function `destroy_empty`



<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_destroy_empty">destroy_empty</a>(self: <a href="external_object.md#0x1_external_object_MovingToStateObject">external_object::MovingToStateObject</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="external_object.md#0x1_external_object_destroy_empty">destroy_empty</a>(self: <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a>) {
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/any_map.md#0x1_any_map_length">any_map::length</a>(&self.resources) == 0, <a href="external_object.md#0x1_external_object_EMOVING_TO_STATE_NOT_FINISHED">EMOVING_TO_STATE_NOT_FINISHED</a>);
    <b>let</b> <a href="external_object.md#0x1_external_object_MovingToStateObject">MovingToStateObject</a> {
        resources: _
    } = self;
}
</code></pre>



</details>

<a id="0x1_external_object_transfer"></a>

## Function `transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="external_object.md#0x1_external_object_transfer">transfer</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="external_object.md#0x1_external_object_transfer">transfer</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>to</b>: <b>address</b>) {
    <b>let</b> <a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a> {
        object_ref,
        resources,
        mut_permission,
    } = <a href="external_unique_state.md#0x1_external_unique_state_move_from_external_storage">external_unique_state::move_from_external_storage</a>&lt;<a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a>, <a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a>&gt;(external_bytes, &<a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> {});

    <b>let</b> constructor_ref = <a href="object.md#0x1_object_create_object_at_address_from_ref">object::create_object_at_address_from_ref</a>(object_ref);

    <a href="object.md#0x1_object_transfer">object::transfer</a>&lt;ObjectCore&gt;(owner, constructor_ref.object_from_constructor_ref(), <b>to</b>);

    <b>let</b> object_ref = <a href="object.md#0x1_object_delete_and_can_recreate">object::delete_and_can_recreate</a>(constructor_ref.generate_delete_and_recreate_ref());
    <b>let</b> compressed_object = <a href="external_object.md#0x1_external_object_ExternalObject">ExternalObject</a> {
        object_ref,
        resources,
        mut_permission,
    };

    <a href="external_unique_state.md#0x1_external_unique_state_move_to_external_storage">external_unique_state::move_to_external_storage</a>(compressed_object, &<a href="external_object.md#0x1_external_object_ExternalObjectWitness">ExternalObjectWitness</a> {});
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
