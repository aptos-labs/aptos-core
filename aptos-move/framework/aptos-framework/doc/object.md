
<a name="0x1_object"></a>

# Module `0x1::object`

This defines the Move object model with the the following properties:
- Simplified storage interface that supports a heterogeneous collection of resources to be
stored together. This enables data types to share a common core data layer (e.g., tokens),
while having richer extensions (e.g., concert ticket, sword).
- Globally accessible data and ownership model that enables creators and developers to dictate
the application and lifetime of data.
- Extensible programming model that supports individualization of user applications that
leverage the core framework including tokens.
- Support emitting events directly, thus improving discoverability of events associated with
objects.
- Considerate of the underlying system by leveraging resource groups for gas efficiency,
avoiding costly deserialization and serialization costs, and supporting deletability.

TODO:
* There is no means to borrow an object or a reference to an object. We are exploring how to
make it so that a reference to a global object can be returned from a function.


-  [Resource `Object`](#0x1_object_Object)
-  [Struct `ObjectGroup`](#0x1_object_ObjectGroup)
-  [Struct `ObjectId`](#0x1_object_ObjectId)
-  [Struct `CreatorRef`](#0x1_object_CreatorRef)
-  [Struct `DeleteRef`](#0x1_object_DeleteRef)
-  [Struct `ExtendRef`](#0x1_object_ExtendRef)
-  [Struct `TransferRef`](#0x1_object_TransferRef)
-  [Struct `LinearTransferRef`](#0x1_object_LinearTransferRef)
-  [Struct `TransferEvent`](#0x1_object_TransferEvent)
-  [Constants](#@Constants_0)
-  [Function `address_to_object_id`](#0x1_object_address_to_object_id)
-  [Function `create_object_id`](#0x1_object_create_object_id)
-  [Function `object_id_address`](#0x1_object_object_id_address)
-  [Function `create_named_object`](#0x1_object_create_named_object)
-  [Function `create_object_from_account`](#0x1_object_create_object_from_account)
-  [Function `create_object_from_object`](#0x1_object_create_object_from_object)
-  [Function `create_object_from_guid`](#0x1_object_create_object_from_guid)
-  [Function `create_object_internal`](#0x1_object_create_object_internal)
-  [Function `generate_delete_ref`](#0x1_object_generate_delete_ref)
-  [Function `generate_extend_ref`](#0x1_object_generate_extend_ref)
-  [Function `generate_transfer_ref`](#0x1_object_generate_transfer_ref)
-  [Function `generate_signer`](#0x1_object_generate_signer)
-  [Function `object_id_from_creator_ref`](#0x1_object_object_id_from_creator_ref)
-  [Function `create_guid`](#0x1_object_create_guid)
-  [Function `new_event_handle`](#0x1_object_new_event_handle)
-  [Function `object_id_from_delete`](#0x1_object_object_id_from_delete)
-  [Function `delete`](#0x1_object_delete)
-  [Function `generate_signer_for_extending`](#0x1_object_generate_signer_for_extending)
-  [Function `disable_ungated_transfer`](#0x1_object_disable_ungated_transfer)
-  [Function `enable_ungated_transfer`](#0x1_object_enable_ungated_transfer)
-  [Function `generate_linear_transfer_ref`](#0x1_object_generate_linear_transfer_ref)
-  [Function `transfer_with_ref`](#0x1_object_transfer_with_ref)
-  [Function `transfer_call`](#0x1_object_transfer_call)
-  [Function `transfer`](#0x1_object_transfer)
-  [Function `transfer_to_object`](#0x1_object_transfer_to_object)
-  [Function `verify_ungated_and_descendant`](#0x1_object_verify_ungated_and_descendant)
-  [Function `owner`](#0x1_object_owner)
-  [Function `is_owner`](#0x1_object_is_owner)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_object_Object"></a>

## Resource `Object`

The core of the object model that defines ownership, transferability, and events.


<pre><code><b>struct</b> <a href="object.md#0x1_object_Object">Object</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>guid_creation_num: u64</code>
</dt>
<dd>
 Used by guid to guarantee globally unique objects and create event streams
</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>
 The address (object or account) that owns this object
</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>
 Object transferring is a common operation, this allows for disabling and enabling
 transfers. Bypassing the use of a the TransferRef.
</dd>
<dt>
<code>transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="object.md#0x1_object_TransferEvent">object::TransferEvent</a>&gt;</code>
</dt>
<dd>
 Emitted events upon transferring of ownership.
</dd>
</dl>


</details>

<a name="0x1_object_ObjectGroup"></a>

## Struct `ObjectGroup`

A shared resource group for storing object resources together in storage.


<pre><code><b>struct</b> <a href="object.md#0x1_object_ObjectGroup">ObjectGroup</a>
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

<a name="0x1_object_ObjectId"></a>

## Struct `ObjectId`

Type safe way of designate an object as at this address.


<pre><code><b>struct</b> <a href="object.md#0x1_object_ObjectId">ObjectId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_object_CreatorRef"></a>

## Struct `CreatorRef`

This is a one time ability given to the creator to configure the object as necessary


<pre><code><b>struct</b> <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>can_delete: bool</code>
</dt>
<dd>
 Set to true so long as deleting the object is possible. For example, the object was
 created via create_named_object.
</dd>
</dl>


</details>

<a name="0x1_object_DeleteRef"></a>

## Struct `DeleteRef`

Used to remove an object from storage.


<pre><code><b>struct</b> <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_object_ExtendRef"></a>

## Struct `ExtendRef`

Used to create events or move additional resources into object storage.


<pre><code><b>struct</b> <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_object_TransferRef"></a>

## Struct `TransferRef`

Used to create LinearTransferRef, hence ownership transfer.


<pre><code><b>struct</b> <a href="object.md#0x1_object_TransferRef">TransferRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_object_LinearTransferRef"></a>

## Struct `LinearTransferRef`

Used to perform transfers. This locks transferring ability to a single time use bound to
the current owner.


<pre><code><b>struct</b> <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_object_TransferEvent"></a>

## Struct `TransferEvent`

Emitted whenever the objects owner field is changed.


<pre><code><b>struct</b> <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_object_ECANNOT_DELETE"></a>

The object does not allow for deletion


<pre><code><b>const</b> <a href="object.md#0x1_object_ECANNOT_DELETE">ECANNOT_DELETE</a>: u64 = 5;
</code></pre>



<a name="0x1_object_EMAXIMUM_NESTING"></a>

Exceeds maximum nesting for an object transfer.


<pre><code><b>const</b> <a href="object.md#0x1_object_EMAXIMUM_NESTING">EMAXIMUM_NESTING</a>: u64 = 6;
</code></pre>



<a name="0x1_object_ENOT_OBJECT_OWNER"></a>

The caller does not have ownership permissions


<pre><code><b>const</b> <a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>: u64 = 4;
</code></pre>



<a name="0x1_object_ENO_UNGATED_TRANSFERS"></a>

The object does not have ungated transfers enabled


<pre><code><b>const</b> <a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>: u64 = 3;
</code></pre>



<a name="0x1_object_EOBJECT_DOES_NOT_EXIST"></a>

An object does not exist at this address


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a name="0x1_object_EOBJECT_EXISTS"></a>

An object already exists at this address


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_EXISTS">EOBJECT_EXISTS</a>: u64 = 1;
</code></pre>



<a name="0x1_object_MAXIMUM_OBJECT_NESTING"></a>

Maximum nesting from one object to another. That is objects can technically have infinte
nesting, but any checks such as transfer will only be evaluated this deep.


<pre><code><b>const</b> <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>: u8 = 8;
</code></pre>



<a name="0x1_object_OBJECT_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object's address. This serves as domain separation to
prevent existing authentication key and resource account derivation to produce an object
address.


<pre><code><b>const</b> <a href="object.md#0x1_object_OBJECT_ADDRESS_SCHEME">OBJECT_ADDRESS_SCHEME</a>: u8 = 254;
</code></pre>



<a name="0x1_object_address_to_object_id"></a>

## Function `address_to_object_id`

Produces an ObjectId from the given address. This is not verified.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_to_object_id">address_to_object_id</a>(object_id: <b>address</b>): <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_to_object_id">address_to_object_id</a>(object_id: <b>address</b>): <a href="object.md#0x1_object_ObjectId">ObjectId</a> {
    <a href="object.md#0x1_object_ObjectId">ObjectId</a> { inner: object_id }
}
</code></pre>



</details>

<a name="0x1_object_create_object_id"></a>

## Function `create_object_id`

Derives an object id from source material: sha3_256([creator address | seed | 0xFE]).
The ObjectId needs to be distinct from create_resource_address


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_id">create_object_id</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_id">create_object_id</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_ObjectId">ObjectId</a> {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(source);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, seed);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_ADDRESS_SCHEME">OBJECT_ADDRESS_SCHEME</a>);
    <a href="object.md#0x1_object_ObjectId">ObjectId</a> { inner: <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes)) }
}
</code></pre>



</details>

<a name="0x1_object_object_id_address"></a>

## Function `object_id_address`

Returns the address of within an ObjectId.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_address">object_id_address</a>(object_id: &<a href="object.md#0x1_object_ObjectId">object::ObjectId</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_address">object_id_address</a>(object_id: &<a href="object.md#0x1_object_ObjectId">ObjectId</a>): <b>address</b> {
    object_id.inner
}
</code></pre>



</details>

<a name="0x1_object_create_named_object"></a>

## Function `create_named_object`

Create a new named object and return the CreatorRef. Named objects can be queried globally
by knowing the user generated seed used to create them. Named objects cannot be deleted.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_named_object">create_named_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_named_object">create_named_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> {
    <b>let</b> creator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> id = <a href="object.md#0x1_object_create_object_id">create_object_id</a>(&creator_address, seed);
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address, id, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_object_create_object_from_account"></a>

## Function `create_object_from_account`

Create a new object from a GUID generated by an account.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_account">create_object_from_account</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_account">create_object_from_account</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> {
    <b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="account.md#0x1_account_create_guid">account::create_guid</a>(creator);
    <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), <a href="guid.md#0x1_guid">guid</a>)
}
</code></pre>



</details>

<a name="0x1_object_create_object_from_object"></a>

## Function `create_object_from_object`

Create a new object from a GUID generated by an object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_object">create_object_from_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_object">create_object_from_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="object.md#0x1_object_create_guid">create_guid</a>(creator);
    <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), <a href="guid.md#0x1_guid">guid</a>)
}
</code></pre>



</details>

<a name="0x1_object_create_object_from_guid"></a>

## Function `create_object_from_guid`



<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(creator_address: <b>address</b>, <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(creator_address: <b>address</b>, <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="guid.md#0x1_guid">guid</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_ADDRESS_SCHEME">OBJECT_ADDRESS_SCHEME</a>);
    <b>let</b> object_id = <a href="object.md#0x1_object_ObjectId">ObjectId</a> { inner: <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes)) };
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address, object_id, <b>true</b>)
}
</code></pre>



</details>

<a name="0x1_object_create_object_internal"></a>

## Function `create_object_internal`



<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address: <b>address</b>, id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>, can_delete: bool): <a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(
    creator_address: <b>address</b>,
    id: <a href="object.md#0x1_object_ObjectId">ObjectId</a>,
    can_delete: bool,
): <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(id.inner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="object.md#0x1_object_EOBJECT_EXISTS">EOBJECT_EXISTS</a>));

    <b>let</b> object_signer = <a href="create_signer.md#0x1_create_signer">create_signer</a>(id.inner);
    <b>let</b> guid_creation_num = 0;
    <b>let</b> transfer_events_guid = <a href="guid.md#0x1_guid_create">guid::create</a>(id.inner, &<b>mut</b> guid_creation_num);

    <b>move_to</b>(
        &object_signer,
        <a href="object.md#0x1_object_Object">Object</a> {
            guid_creation_num,
            owner: creator_address,
            allow_ungated_transfer: <b>true</b>,
            transfer_events: <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(transfer_events_guid),
        },
    );
    <a href="object.md#0x1_object_CreatorRef">CreatorRef</a> { self: id, can_delete }
}
</code></pre>



</details>

<a name="0x1_object_generate_delete_ref"></a>

## Function `generate_delete_ref`

Generates the DeleteRef, which can be used to remove Object from global storage.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_delete_ref">generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>): <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_delete_ref">generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">CreatorRef</a>): <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> {
    <b>assert</b>!(ref.can_delete, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ECANNOT_DELETE">ECANNOT_DELETE</a>));
    <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> { self: ref.self }
}
</code></pre>



</details>

<a name="0x1_object_generate_extend_ref"></a>

## Function `generate_extend_ref`

Generates the ExtendRef, which can be used to add new events and resources to the object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_extend_ref">generate_extend_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>): <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_extend_ref">generate_extend_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">CreatorRef</a>): <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> {
    <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> { self: ref.self }
}
</code></pre>



</details>

<a name="0x1_object_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Generates the TransferRef, which can be used to manage object transfers.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_transfer_ref">generate_transfer_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>): <a href="object.md#0x1_object_TransferRef">object::TransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_transfer_ref">generate_transfer_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">CreatorRef</a>): <a href="object.md#0x1_object_TransferRef">TransferRef</a> {
    <a href="object.md#0x1_object_TransferRef">TransferRef</a> { self: ref.self }
}
</code></pre>



</details>

<a name="0x1_object_generate_signer"></a>

## Function `generate_signer`

Create a signer for the CreatorRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer">generate_signer</a>(ref: &<a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer">generate_signer</a>(ref: &<a href="object.md#0x1_object_CreatorRef">CreatorRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(ref.self.inner)
}
</code></pre>



</details>

<a name="0x1_object_object_id_from_creator_ref"></a>

## Function `object_id_from_creator_ref`

Returns the address of within a CreatorRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_from_creator_ref">object_id_from_creator_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">object::CreatorRef</a>): <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_from_creator_ref">object_id_from_creator_ref</a>(ref: &<a href="object.md#0x1_object_CreatorRef">CreatorRef</a>): <a href="object.md#0x1_object_ObjectId">ObjectId</a> {
    ref.self
}
</code></pre>



</details>

<a name="0x1_object_create_guid"></a>

## Function `create_guid`

Create a guid for the object, typically used for events


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a> <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>);
    <b>let</b> object_data = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(addr);
    <a href="guid.md#0x1_guid_create">guid::create</a>(addr, &<b>mut</b> object_data.guid_creation_num)
}
</code></pre>



</details>

<a name="0x1_object_new_event_handle"></a>

## Function `new_event_handle`

Generate a new event handle.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_new_event_handle">new_event_handle</a>&lt;T: drop + store&gt;(
    <a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt; <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(<a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>))
}
</code></pre>



</details>

<a name="0x1_object_object_id_from_delete"></a>

## Function `object_id_from_delete`

Returns the address of within a DeleteRef.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_from_delete">object_id_from_delete</a>(ref: &<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>): <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_id_from_delete">object_id_from_delete</a>(ref: &<a href="object.md#0x1_object_DeleteRef">DeleteRef</a>): <a href="object.md#0x1_object_ObjectId">ObjectId</a> {
    ref.self
}
</code></pre>



</details>

<a name="0x1_object_delete"></a>

## Function `delete`

Removes from the specified Object from global storage.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_delete">delete</a>(ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_delete">delete</a>(ref: <a href="object.md#0x1_object_DeleteRef">DeleteRef</a>) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>move_from</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(ref.self.inner);
    <b>let</b> <a href="object.md#0x1_object_Object">Object</a> {
        guid_creation_num: _,
        owner: _,
        allow_ungated_transfer: _,
        transfer_events,
    } = <a href="object.md#0x1_object">object</a>;
    <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(transfer_events);
}
</code></pre>



</details>

<a name="0x1_object_generate_signer_for_extending"></a>

## Function `generate_signer_for_extending`

Create a signer for the ExtendRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer_for_extending">generate_signer_for_extending</a>(ref: &<a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer_for_extending">generate_signer_for_extending</a>(ref: &<a href="object.md#0x1_object_ExtendRef">ExtendRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(ref.self.inner)
}
</code></pre>



</details>

<a name="0x1_object_disable_ungated_transfer"></a>

## Function `disable_ungated_transfer`

Disable direct transfer, transfers can only be triggered via a TransferRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_disable_ungated_transfer">disable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_disable_ungated_transfer">disable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">TransferRef</a>) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(ref.self.inner);
    <a href="object.md#0x1_object">object</a>.allow_ungated_transfer = <b>false</b>;
}
</code></pre>



</details>

<a name="0x1_object_enable_ungated_transfer"></a>

## Function `enable_ungated_transfer`

Enable direct transfer.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_enable_ungated_transfer">enable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_enable_ungated_transfer">enable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">TransferRef</a>) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(ref.self.inner);
    <a href="object.md#0x1_object">object</a>.allow_ungated_transfer = <b>true</b>;
}
</code></pre>



</details>

<a name="0x1_object_generate_linear_transfer_ref"></a>

## Function `generate_linear_transfer_ref`

Create a LinearTransferRef for a one-time transfer. This requires that the owner at the
time of generation is the owner at the time of transferring.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_linear_transfer_ref">generate_linear_transfer_ref</a>(ref: <a href="object.md#0x1_object_TransferRef">object::TransferRef</a>): <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_linear_transfer_ref">generate_linear_transfer_ref</a>(ref: <a href="object.md#0x1_object_TransferRef">TransferRef</a>): <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> owner = <a href="object.md#0x1_object_owner">owner</a>(ref.self);
    <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> {
        self: ref.self,
        owner,
    }
}
</code></pre>



</details>

<a name="0x1_object_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer to the destination address using a LinearTransferRef.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_with_ref">transfer_with_ref</a>(ref: <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_with_ref">transfer_with_ref</a>(ref: <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a>, <b>to</b>: <b>address</b>) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(ref.self.inner);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="object.md#0x1_object">object</a>.transfer_events,
        <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> {
            object_id: ref.self,
            from: <a href="object.md#0x1_object">object</a>.owner,
            <b>to</b>,
        },
    );
    <a href="object.md#0x1_object">object</a>.owner = <b>to</b>;
}
</code></pre>



</details>

<a name="0x1_object_transfer_call"></a>

## Function `transfer_call`

Entry function that can be used to transfer, if allow_ungated_transfer is set true.


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_call">transfer_call</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, object_id: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_call">transfer_call</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    object_id: <b>address</b>,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <a href="object.md#0x1_object_transfer">transfer</a>(owner, <a href="object.md#0x1_object_ObjectId">ObjectId</a> { inner: object_id }, <b>to</b>)
}
</code></pre>



</details>

<a name="0x1_object_transfer"></a>

## Function `transfer`

Transfers the given object if allow_ungated_transfer is set true. Note, that this allows
the owner of a nested object to transfer that object, so long as allow_ungated_transfer is
enabled at each stage in the hierarchy.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer">transfer</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, object_id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer">transfer</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    object_id: <a href="object.md#0x1_object_ObjectId">ObjectId</a>,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner_address, object_id.inner);

    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(object_id.inner);
    <b>if</b> (<a href="object.md#0x1_object">object</a>.owner == <b>to</b>) {
        <b>return</b>
    };

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="object.md#0x1_object">object</a>.transfer_events,
        <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> {
            object_id: object_id,
            from: <a href="object.md#0x1_object">object</a>.owner,
            <b>to</b>,
        },
    );
    <a href="object.md#0x1_object">object</a>.owner = <b>to</b>;
}
</code></pre>



</details>

<a name="0x1_object_transfer_to_object"></a>

## Function `transfer_to_object`

Transfer the given object to another object. See <code>transfer</code> for more information.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_to_object">transfer_to_object</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, object_id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>, <b>to</b>: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_to_object">transfer_to_object</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    object_id: <a href="object.md#0x1_object_ObjectId">ObjectId</a>,
    <b>to</b>: <a href="object.md#0x1_object_ObjectId">ObjectId</a>,
) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <a href="object.md#0x1_object_transfer">transfer</a>(owner, object_id, <b>to</b>.inner)
}
</code></pre>



</details>

<a name="0x1_object_verify_ungated_and_descendant"></a>

## Function `verify_ungated_and_descendant`

This checks that the destination address is eventually owned by the owner and that each
object between the two allows for ungated transfers. Note, this is limited to a depth of 8
objects may have cyclic dependencies.


<pre><code><b>fun</b> <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner: <b>address</b>, destination: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner: <b>address</b>, destination: <b>address</b>) <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>let</b> current_address = destination;
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(current_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );

    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(current_address);
    <b>assert</b>!(
        <a href="object.md#0x1_object">object</a>.allow_ungated_transfer,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>),
    );

    <b>let</b> current_address = <a href="object.md#0x1_object">object</a>.owner;

    <b>let</b> count = 0;
    <b>while</b> (owner != current_address) {
        <b>let</b> count = count + 1;
        <b>assert</b>!(count &lt; <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="object.md#0x1_object_EMAXIMUM_NESTING">EMAXIMUM_NESTING</a>));

        // At this point, the first <a href="object.md#0x1_object">object</a> <b>exists</b> and so the more likely case is that the
        // <a href="object.md#0x1_object">object</a>'s owner is not an <a href="object.md#0x1_object">object</a>. So we <b>return</b> a more sensible <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>.
        <b>assert</b>!(
            <b>exists</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(current_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>),
        );
        <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(current_address);
        <b>assert</b>!(
            <a href="object.md#0x1_object">object</a>.allow_ungated_transfer,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>),
        );

        current_address = <a href="object.md#0x1_object">object</a>.owner;
    };
}
</code></pre>



</details>

<a name="0x1_object_owner"></a>

## Function `owner`

Accessors
Return the current owner.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owner">owner</a>(object_id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owner">owner</a>(object_id: <a href="object.md#0x1_object_ObjectId">ObjectId</a>): <b>address</b> <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(object_id.inner),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );
    <b>borrow_global</b>&lt;<a href="object.md#0x1_object_Object">Object</a>&gt;(object_id.inner).owner
}
</code></pre>



</details>

<a name="0x1_object_is_owner"></a>

## Function `is_owner`

Return true if the provided address is the current owner.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_owner">is_owner</a>(object_id: <a href="object.md#0x1_object_ObjectId">object::ObjectId</a>, owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_owner">is_owner</a>(object_id: <a href="object.md#0x1_object_ObjectId">ObjectId</a>, owner: <b>address</b>): bool <b>acquires</b> <a href="object.md#0x1_object_Object">Object</a> {
    <a href="object.md#0x1_object_owner">owner</a>(object_id) == owner
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
