
<a id="0x1_object"></a>

# Module `0x1::object`

This defines the Move object model with the following properties:<br/> &#45; Simplified storage interface that supports a heterogeneous collection of resources to be<br/>   stored together. This enables data types to share a common core data layer (e.g., tokens),<br/>   while having richer extensions (e.g., concert ticket, sword).<br/> &#45; Globally accessible data and ownership model that enables creators and developers to dictate<br/>   the application and lifetime of data.<br/> &#45; Extensible programming model that supports individualization of user applications that<br/>   leverage the core framework including tokens.<br/> &#45; Support emitting events directly, thus improving discoverability of events associated with<br/>   objects.<br/> &#45; Considerate of the underlying system by leveraging resource groups for gas efficiency,<br/>   avoiding costly deserialization and serialization costs, and supporting deletability.<br/><br/> TODO:<br/> &#42; There is no means to borrow an object or a reference to an object. We are exploring how to<br/>   make it so that a reference to a global object can be returned from a function.


-  [Resource `ObjectCore`](#0x1_object_ObjectCore)
-  [Resource `TombStone`](#0x1_object_TombStone)
-  [Resource `Untransferable`](#0x1_object_Untransferable)
-  [Struct `ObjectGroup`](#0x1_object_ObjectGroup)
-  [Struct `Object`](#0x1_object_Object)
-  [Struct `ConstructorRef`](#0x1_object_ConstructorRef)
-  [Struct `DeleteRef`](#0x1_object_DeleteRef)
-  [Struct `ExtendRef`](#0x1_object_ExtendRef)
-  [Struct `TransferRef`](#0x1_object_TransferRef)
-  [Struct `LinearTransferRef`](#0x1_object_LinearTransferRef)
-  [Struct `DeriveRef`](#0x1_object_DeriveRef)
-  [Struct `TransferEvent`](#0x1_object_TransferEvent)
-  [Struct `Transfer`](#0x1_object_Transfer)
-  [Constants](#@Constants_0)
-  [Function `is_untransferable`](#0x1_object_is_untransferable)
-  [Function `is_burnt`](#0x1_object_is_burnt)
-  [Function `address_to_object`](#0x1_object_address_to_object)
-  [Function `is_object`](#0x1_object_is_object)
-  [Function `object_exists`](#0x1_object_object_exists)
-  [Function `create_object_address`](#0x1_object_create_object_address)
-  [Function `create_user_derived_object_address_impl`](#0x1_object_create_user_derived_object_address_impl)
-  [Function `create_user_derived_object_address`](#0x1_object_create_user_derived_object_address)
-  [Function `create_guid_object_address`](#0x1_object_create_guid_object_address)
-  [Function `exists_at`](#0x1_object_exists_at)
-  [Function `object_address`](#0x1_object_object_address)
-  [Function `convert`](#0x1_object_convert)
-  [Function `create_named_object`](#0x1_object_create_named_object)
-  [Function `create_user_derived_object`](#0x1_object_create_user_derived_object)
-  [Function `create_object`](#0x1_object_create_object)
-  [Function `create_sticky_object`](#0x1_object_create_sticky_object)
-  [Function `create_sticky_object_at_address`](#0x1_object_create_sticky_object_at_address)
-  [Function `create_object_from_account`](#0x1_object_create_object_from_account)
-  [Function `create_object_from_object`](#0x1_object_create_object_from_object)
-  [Function `create_object_from_guid`](#0x1_object_create_object_from_guid)
-  [Function `create_object_internal`](#0x1_object_create_object_internal)
-  [Function `generate_delete_ref`](#0x1_object_generate_delete_ref)
-  [Function `generate_extend_ref`](#0x1_object_generate_extend_ref)
-  [Function `generate_transfer_ref`](#0x1_object_generate_transfer_ref)
-  [Function `generate_derive_ref`](#0x1_object_generate_derive_ref)
-  [Function `generate_signer`](#0x1_object_generate_signer)
-  [Function `address_from_constructor_ref`](#0x1_object_address_from_constructor_ref)
-  [Function `object_from_constructor_ref`](#0x1_object_object_from_constructor_ref)
-  [Function `can_generate_delete_ref`](#0x1_object_can_generate_delete_ref)
-  [Function `create_guid`](#0x1_object_create_guid)
-  [Function `new_event_handle`](#0x1_object_new_event_handle)
-  [Function `address_from_delete_ref`](#0x1_object_address_from_delete_ref)
-  [Function `object_from_delete_ref`](#0x1_object_object_from_delete_ref)
-  [Function `delete`](#0x1_object_delete)
-  [Function `generate_signer_for_extending`](#0x1_object_generate_signer_for_extending)
-  [Function `address_from_extend_ref`](#0x1_object_address_from_extend_ref)
-  [Function `disable_ungated_transfer`](#0x1_object_disable_ungated_transfer)
-  [Function `set_untransferable`](#0x1_object_set_untransferable)
-  [Function `enable_ungated_transfer`](#0x1_object_enable_ungated_transfer)
-  [Function `generate_linear_transfer_ref`](#0x1_object_generate_linear_transfer_ref)
-  [Function `transfer_with_ref`](#0x1_object_transfer_with_ref)
-  [Function `transfer_call`](#0x1_object_transfer_call)
-  [Function `transfer`](#0x1_object_transfer)
-  [Function `transfer_raw`](#0x1_object_transfer_raw)
-  [Function `transfer_raw_inner`](#0x1_object_transfer_raw_inner)
-  [Function `transfer_to_object`](#0x1_object_transfer_to_object)
-  [Function `verify_ungated_and_descendant`](#0x1_object_verify_ungated_and_descendant)
-  [Function `burn`](#0x1_object_burn)
-  [Function `unburn`](#0x1_object_unburn)
-  [Function `ungated_transfer_allowed`](#0x1_object_ungated_transfer_allowed)
-  [Function `owner`](#0x1_object_owner)
-  [Function `is_owner`](#0x1_object_is_owner)
-  [Function `owns`](#0x1_object_owns)
-  [Function `root_owner`](#0x1_object_root_owner)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `address_to_object`](#@Specification_1_address_to_object)
    -  [Function `create_object_address`](#@Specification_1_create_object_address)
    -  [Function `create_user_derived_object_address_impl`](#@Specification_1_create_user_derived_object_address_impl)
    -  [Function `create_user_derived_object_address`](#@Specification_1_create_user_derived_object_address)
    -  [Function `create_guid_object_address`](#@Specification_1_create_guid_object_address)
    -  [Function `exists_at`](#@Specification_1_exists_at)
    -  [Function `object_address`](#@Specification_1_object_address)
    -  [Function `convert`](#@Specification_1_convert)
    -  [Function `create_named_object`](#@Specification_1_create_named_object)
    -  [Function `create_user_derived_object`](#@Specification_1_create_user_derived_object)
    -  [Function `create_object`](#@Specification_1_create_object)
    -  [Function `create_sticky_object`](#@Specification_1_create_sticky_object)
    -  [Function `create_sticky_object_at_address`](#@Specification_1_create_sticky_object_at_address)
    -  [Function `create_object_from_account`](#@Specification_1_create_object_from_account)
    -  [Function `create_object_from_object`](#@Specification_1_create_object_from_object)
    -  [Function `create_object_from_guid`](#@Specification_1_create_object_from_guid)
    -  [Function `create_object_internal`](#@Specification_1_create_object_internal)
    -  [Function `generate_delete_ref`](#@Specification_1_generate_delete_ref)
    -  [Function `generate_transfer_ref`](#@Specification_1_generate_transfer_ref)
    -  [Function `object_from_constructor_ref`](#@Specification_1_object_from_constructor_ref)
    -  [Function `create_guid`](#@Specification_1_create_guid)
    -  [Function `new_event_handle`](#@Specification_1_new_event_handle)
    -  [Function `object_from_delete_ref`](#@Specification_1_object_from_delete_ref)
    -  [Function `delete`](#@Specification_1_delete)
    -  [Function `disable_ungated_transfer`](#@Specification_1_disable_ungated_transfer)
    -  [Function `set_untransferable`](#@Specification_1_set_untransferable)
    -  [Function `enable_ungated_transfer`](#@Specification_1_enable_ungated_transfer)
    -  [Function `generate_linear_transfer_ref`](#@Specification_1_generate_linear_transfer_ref)
    -  [Function `transfer_with_ref`](#@Specification_1_transfer_with_ref)
    -  [Function `transfer_call`](#@Specification_1_transfer_call)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `transfer_raw`](#@Specification_1_transfer_raw)
    -  [Function `transfer_to_object`](#@Specification_1_transfer_to_object)
    -  [Function `verify_ungated_and_descendant`](#@Specification_1_verify_ungated_and_descendant)
    -  [Function `burn`](#@Specification_1_burn)
    -  [Function `unburn`](#@Specification_1_unburn)
    -  [Function `ungated_transfer_allowed`](#@Specification_1_ungated_transfer_allowed)
    -  [Function `owner`](#@Specification_1_owner)
    -  [Function `is_owner`](#@Specification_1_is_owner)
    -  [Function `owns`](#@Specification_1_owns)
    -  [Function `root_owner`](#@Specification_1_root_owner)


<pre><code>use 0x1::account;<br/>use 0x1::bcs;<br/>use 0x1::create_signer;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::from_bcs;<br/>use 0x1::guid;<br/>use 0x1::hash;<br/>use 0x1::signer;<br/>use 0x1::transaction_context;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_object_ObjectCore"></a>

## Resource `ObjectCore`

The core of the object model that defines ownership, transferability, and events.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct ObjectCore has key<br/></code></pre>



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
<code>owner: address</code>
</dt>
<dd>
 The address (object or account) that owns this object
</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>
 Object transferring is a common operation, this allows for disabling and enabling<br/> transfers bypassing the use of a TransferRef.
</dd>
<dt>
<code>transfer_events: event::EventHandle&lt;object::TransferEvent&gt;</code>
</dt>
<dd>
 Emitted events upon transferring of ownership.
</dd>
</dl>


</details>

<a id="0x1_object_TombStone"></a>

## Resource `TombStone`

This is added to objects that are burnt (ownership transferred to BURN_ADDRESS).


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct TombStone has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>original_owner: address</code>
</dt>
<dd>
 Track the previous owner before the object is burnt so they can reclaim later if so desired.
</dd>
</dl>


</details>

<a id="0x1_object_Untransferable"></a>

## Resource `Untransferable`

The existence of this renders all <code>TransferRef</code>s irrelevant. The object cannot be moved.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Untransferable has key<br/></code></pre>



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

<a id="0x1_object_ObjectGroup"></a>

## Struct `ObjectGroup`

A shared resource group for storing object resources together in storage.


<pre><code>&#35;[resource_group(&#35;[scope &#61; global])]<br/>struct ObjectGroup<br/></code></pre>



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

<a id="0x1_object_Object"></a>

## Struct `Object`

A pointer to an object &#45;&#45; these can only provide guarantees based upon the underlying data<br/> type, that is the validity of T existing at an address is something that cannot be verified<br/> by any other module than the module that defined T. Similarly, the module that defines T<br/> can remove it from storage at any point in time.


<pre><code>struct Object&lt;T&gt; has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_ConstructorRef"></a>

## Struct `ConstructorRef`

This is a one time ability given to the creator to configure the object as necessary


<pre><code>struct ConstructorRef has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: address</code>
</dt>
<dd>

</dd>
<dt>
<code>can_delete: bool</code>
</dt>
<dd>
 True if the object can be deleted. Named objects are not deletable.
</dd>
</dl>


</details>

<a id="0x1_object_DeleteRef"></a>

## Struct `DeleteRef`

Used to remove an object from storage.


<pre><code>struct DeleteRef has drop, store<br/></code></pre>



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

<a id="0x1_object_ExtendRef"></a>

## Struct `ExtendRef`

Used to create events or move additional resources into object storage.


<pre><code>struct ExtendRef has drop, store<br/></code></pre>



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

<a id="0x1_object_TransferRef"></a>

## Struct `TransferRef`

Used to create LinearTransferRef, hence ownership transfer.


<pre><code>struct TransferRef has drop, store<br/></code></pre>



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

<a id="0x1_object_LinearTransferRef"></a>

## Struct `LinearTransferRef`

Used to perform transfers. This locks transferring ability to a single time use bound to<br/> the current owner.


<pre><code>struct LinearTransferRef has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: address</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_DeriveRef"></a>

## Struct `DeriveRef`

Used to create derived objects from a given objects.


<pre><code>struct DeriveRef has drop, store<br/></code></pre>



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

<a id="0x1_object_TransferEvent"></a>

## Struct `TransferEvent`

Emitted whenever the object&apos;s owner field is changed.


<pre><code>struct TransferEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object: address</code>
</dt>
<dd>

</dd>
<dt>
<code>from: address</code>
</dt>
<dd>

</dd>
<dt>
<code>to: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_Transfer"></a>

## Struct `Transfer`

Emitted whenever the object&apos;s owner field is changed.


<pre><code>&#35;[event]<br/>struct Transfer has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object: address</code>
</dt>
<dd>

</dd>
<dt>
<code>from: address</code>
</dt>
<dd>

</dd>
<dt>
<code>to: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_object_BURN_ADDRESS"></a>

Address where unwanted objects can be forcefully transferred to.


<pre><code>const BURN_ADDRESS: address &#61; 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;<br/></code></pre>



<a id="0x1_object_DERIVE_AUID_ADDRESS_SCHEME"></a>

generate_unique_address uses this for domain separation within its native implementation


<pre><code>const DERIVE_AUID_ADDRESS_SCHEME: u8 &#61; 251;<br/></code></pre>



<a id="0x1_object_ECANNOT_DELETE"></a>

The object does not allow for deletion


<pre><code>const ECANNOT_DELETE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_object_EMAXIMUM_NESTING"></a>

Exceeds maximum nesting for an object transfer.


<pre><code>const EMAXIMUM_NESTING: u64 &#61; 6;<br/></code></pre>



<a id="0x1_object_ENOT_MOVABLE"></a>

Object is untransferable any operations that might result in a transfer are disallowed.


<pre><code>const ENOT_MOVABLE: u64 &#61; 9;<br/></code></pre>



<a id="0x1_object_ENOT_OBJECT_OWNER"></a>

The caller does not have ownership permissions


<pre><code>const ENOT_OBJECT_OWNER: u64 &#61; 4;<br/></code></pre>



<a id="0x1_object_ENO_UNGATED_TRANSFERS"></a>

The object does not have ungated transfers enabled


<pre><code>const ENO_UNGATED_TRANSFERS: u64 &#61; 3;<br/></code></pre>



<a id="0x1_object_EOBJECT_DOES_NOT_EXIST"></a>

An object does not exist at this address


<pre><code>const EOBJECT_DOES_NOT_EXIST: u64 &#61; 2;<br/></code></pre>



<a id="0x1_object_EOBJECT_EXISTS"></a>

An object already exists at this address


<pre><code>const EOBJECT_EXISTS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_object_EOBJECT_NOT_BURNT"></a>

Cannot reclaim objects that weren&apos;t burnt.


<pre><code>const EOBJECT_NOT_BURNT: u64 &#61; 8;<br/></code></pre>



<a id="0x1_object_ERESOURCE_DOES_NOT_EXIST"></a>

The resource is not stored at the specified address.


<pre><code>const ERESOURCE_DOES_NOT_EXIST: u64 &#61; 7;<br/></code></pre>



<a id="0x1_object_INIT_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code>const INIT_GUID_CREATION_NUM: u64 &#61; 1125899906842624;<br/></code></pre>



<a id="0x1_object_MAXIMUM_OBJECT_NESTING"></a>

Maximum nesting from one object to another. That is objects can technically have infinte<br/> nesting, but any checks such as transfer will only be evaluated this deep.


<pre><code>const MAXIMUM_OBJECT_NESTING: u8 &#61; 8;<br/></code></pre>



<a id="0x1_object_OBJECT_DERIVED_SCHEME"></a>

Scheme identifier used to generate an object&apos;s address <code>obj_addr</code> as derived from another object.<br/> The object&apos;s address is generated as:<br/> ```<br/>     obj_addr &#61; sha3_256(account addr &#124; derived from object&apos;s address &#124; 0xFC)<br/> ```<br/><br/> This 0xFC constant serves as a domain separation tag to prevent existing authentication key and resource account<br/> derivation to produce an object address.


<pre><code>const OBJECT_DERIVED_SCHEME: u8 &#61; 252;<br/></code></pre>



<a id="0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object&apos;s address <code>obj_addr</code> via a fresh GUID generated by the creator at<br/> <code>source_addr</code>. The object&apos;s address is generated as:<br/> ```<br/>     obj_addr &#61; sha3_256(guid &#124; 0xFD)<br/> ```<br/> where <code>guid &#61; account::create_guid(create_signer(source_addr))</code><br/><br/> This 0xFD constant serves as a domain separation tag to prevent existing authentication key and resource account<br/> derivation to produce an object address.


<pre><code>const OBJECT_FROM_GUID_ADDRESS_SCHEME: u8 &#61; 253;<br/></code></pre>



<a id="0x1_object_OBJECT_FROM_SEED_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object&apos;s address <code>obj_addr</code> from the creator&apos;s <code>source_addr</code> and a <code>seed</code> as:<br/>     obj_addr &#61; sha3_256(source_addr &#124; seed &#124; 0xFE).<br/><br/> This 0xFE constant serves as a domain separation tag to prevent existing authentication key and resource account<br/> derivation to produce an object address.


<pre><code>const OBJECT_FROM_SEED_ADDRESS_SCHEME: u8 &#61; 254;<br/></code></pre>



<a id="0x1_object_is_untransferable"></a>

## Function `is_untransferable`



<pre><code>&#35;[view]<br/>public fun is_untransferable&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_untransferable&lt;T: key&gt;(object: Object&lt;T&gt;): bool &#123;<br/>    exists&lt;Untransferable&gt;(object.inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_is_burnt"></a>

## Function `is_burnt`



<pre><code>&#35;[view]<br/>public fun is_burnt&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_burnt&lt;T: key&gt;(object: Object&lt;T&gt;): bool &#123;<br/>    exists&lt;TombStone&gt;(object.inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_address_to_object"></a>

## Function `address_to_object`

Produces an ObjectId from the given address. This is not verified.


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): object::Object&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): Object&lt;T&gt; &#123;<br/>    assert!(exists&lt;ObjectCore&gt;(object), error::not_found(EOBJECT_DOES_NOT_EXIST));<br/>    assert!(exists_at&lt;T&gt;(object), error::not_found(ERESOURCE_DOES_NOT_EXIST));<br/>    Object&lt;T&gt; &#123; inner: object &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_is_object"></a>

## Function `is_object`

Returns true if there exists an object or the remnants of an object.


<pre><code>public fun is_object(object: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_object(object: address): bool &#123;<br/>    exists&lt;ObjectCore&gt;(object)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_object_exists"></a>

## Function `object_exists`

Returns true if there exists an object with resource T.


<pre><code>public fun object_exists&lt;T: key&gt;(object: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_exists&lt;T: key&gt;(object: address): bool &#123;<br/>    exists&lt;ObjectCore&gt;(object) &amp;&amp; exists_at&lt;T&gt;(object)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object_address"></a>

## Function `create_object_address`

Derives an object address from source material: sha3_256([creator address &#124; seed &#124; 0xFE]).


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address &#123;<br/>    let bytes &#61; bcs::to_bytes(source);<br/>    vector::append(&amp;mut bytes, seed);<br/>    vector::push_back(&amp;mut bytes, OBJECT_FROM_SEED_ADDRESS_SCHEME);<br/>    from_bcs::to_address(hash::sha3_256(bytes))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address_impl"></a>

## Function `create_user_derived_object_address_impl`



<pre><code>fun create_user_derived_object_address_impl(source: address, derive_from: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun create_user_derived_object_address_impl(source: address, derive_from: address): address;<br/></code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address"></a>

## Function `create_user_derived_object_address`

Derives an object address from the source address and an object: sha3_256([source &#124; object addr &#124; 0xFC]).


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address &#123;<br/>    if (std::features::object_native_derived_address_enabled()) &#123;<br/>        create_user_derived_object_address_impl(source, derive_from)<br/>    &#125; else &#123;<br/>        let bytes &#61; bcs::to_bytes(&amp;source);<br/>        vector::append(&amp;mut bytes, bcs::to_bytes(&amp;derive_from));<br/>        vector::push_back(&amp;mut bytes, OBJECT_DERIVED_SCHEME);<br/>        from_bcs::to_address(hash::sha3_256(bytes))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_guid_object_address"></a>

## Function `create_guid_object_address`

Derives an object from an Account GUID.


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address &#123;<br/>    let id &#61; guid::create_id(source, creation_num);<br/>    let bytes &#61; bcs::to_bytes(&amp;id);<br/>    vector::push_back(&amp;mut bytes, OBJECT_FROM_GUID_ADDRESS_SCHEME);<br/>    from_bcs::to_address(hash::sha3_256(bytes))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_exists_at"></a>

## Function `exists_at`



<pre><code>fun exists_at&lt;T: key&gt;(object: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun exists_at&lt;T: key&gt;(object: address): bool;<br/></code></pre>



</details>

<a id="0x1_object_object_address"></a>

## Function `object_address`

Returns the address of within an ObjectId.


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;Object&lt;T&gt;): address &#123;<br/>    object.inner<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_convert"></a>

## Function `convert`

Convert Object&lt;X&gt; to Object&lt;Y&gt;.


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: object::Object&lt;X&gt;): object::Object&lt;Y&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: Object&lt;X&gt;): Object&lt;Y&gt; &#123;<br/>    address_to_object&lt;Y&gt;(object.inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_named_object"></a>

## Function `create_named_object`

Create a new named object and return the ConstructorRef. Named objects can be queried globally<br/> by knowing the user generated seed used to create them. Named objects cannot be deleted.


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let obj_addr &#61; create_object_address(&amp;creator_address, seed);<br/>    create_object_internal(creator_address, obj_addr, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_user_derived_object"></a>

## Function `create_user_derived_object`

Create a new object whose address is derived based on the creator account address and another object.<br/> Derivde objects, similar to named objects, cannot be deleted.


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;object::DeriveRef): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;DeriveRef): ConstructorRef &#123;<br/>    let obj_addr &#61; create_user_derived_object_address(creator_address, derive_ref.self);<br/>    create_object_internal(creator_address, obj_addr, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object"></a>

## Function `create_object`

Create a new object by generating a random unique address based on transaction hash.<br/> The unique address is computed sha3_256([transaction hash &#124; auid counter &#124; 0xFB]).<br/> The created object is deletable as we can guarantee the same unique address can<br/> never be regenerated with future txs.


<pre><code>public fun create_object(owner_address: address): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object(owner_address: address): ConstructorRef &#123;<br/>    let unique_address &#61; transaction_context::generate_auid_address();<br/>    create_object_internal(owner_address, unique_address, true)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_sticky_object"></a>

## Function `create_sticky_object`

Same as <code>create_object</code> except the object to be created will be undeletable.


<pre><code>public fun create_sticky_object(owner_address: address): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_sticky_object(owner_address: address): ConstructorRef &#123;<br/>    let unique_address &#61; transaction_context::generate_auid_address();<br/>    create_object_internal(owner_address, unique_address, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_sticky_object_at_address"></a>

## Function `create_sticky_object_at_address`

Create a sticky object at a specific address. Only used by aptos_framework::coin.


<pre><code>public(friend) fun create_sticky_object_at_address(owner_address: address, object_address: address): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_sticky_object_at_address(<br/>    owner_address: address,<br/>    object_address: address,<br/>): ConstructorRef &#123;<br/>    create_object_internal(owner_address, object_address, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object_from_account"></a>

## Function `create_object_from_account`

Use <code>create_object</code> instead.<br/> Create a new object from a GUID generated by an account.<br/> As the GUID creation internally increments a counter, two transactions that executes<br/> <code>create_object_from_account</code> function for the same creator run sequentially.<br/> Therefore, using <code>create_object</code> method for creating objects is preferrable as it<br/> doesn&apos;t have the same bottlenecks.


<pre><code>&#35;[deprecated]<br/>public fun create_object_from_account(creator: &amp;signer): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_from_account(creator: &amp;signer): ConstructorRef &#123;<br/>    let guid &#61; account::create_guid(creator);<br/>    create_object_from_guid(signer::address_of(creator), guid)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object_from_object"></a>

## Function `create_object_from_object`

Use <code>create_object</code> instead.<br/> Create a new object from a GUID generated by an object.<br/> As the GUID creation internally increments a counter, two transactions that executes<br/> <code>create_object_from_object</code> function for the same creator run sequentially.<br/> Therefore, using <code>create_object</code> method for creating objects is preferrable as it<br/> doesn&apos;t have the same bottlenecks.


<pre><code>&#35;[deprecated]<br/>public fun create_object_from_object(creator: &amp;signer): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_from_object(creator: &amp;signer): ConstructorRef acquires ObjectCore &#123;<br/>    let guid &#61; create_guid(creator);<br/>    create_object_from_guid(signer::address_of(creator), guid)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object_from_guid"></a>

## Function `create_object_from_guid`



<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef &#123;<br/>    let bytes &#61; bcs::to_bytes(&amp;guid);<br/>    vector::push_back(&amp;mut bytes, OBJECT_FROM_GUID_ADDRESS_SCHEME);<br/>    let obj_addr &#61; from_bcs::to_address(hash::sha3_256(bytes));<br/>    create_object_internal(creator_address, obj_addr, true)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_object_internal"></a>

## Function `create_object_internal`



<pre><code>fun create_object_internal(creator_address: address, object: address, can_delete: bool): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_object_internal(<br/>    creator_address: address,<br/>    object: address,<br/>    can_delete: bool,<br/>): ConstructorRef &#123;<br/>    assert!(!exists&lt;ObjectCore&gt;(object), error::already_exists(EOBJECT_EXISTS));<br/><br/>    let object_signer &#61; create_signer(object);<br/>    let guid_creation_num &#61; INIT_GUID_CREATION_NUM;<br/>    let transfer_events_guid &#61; guid::create(object, &amp;mut guid_creation_num);<br/><br/>    move_to(<br/>        &amp;object_signer,<br/>        ObjectCore &#123;<br/>            guid_creation_num,<br/>            owner: creator_address,<br/>            allow_ungated_transfer: true,<br/>            transfer_events: event::new_event_handle(transfer_events_guid),<br/>        &#125;,<br/>    );<br/>    ConstructorRef &#123; self: object, can_delete &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_delete_ref"></a>

## Function `generate_delete_ref`

Generates the DeleteRef, which can be used to remove ObjectCore from global storage.


<pre><code>public fun generate_delete_ref(ref: &amp;object::ConstructorRef): object::DeleteRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_delete_ref(ref: &amp;ConstructorRef): DeleteRef &#123;<br/>    assert!(ref.can_delete, error::permission_denied(ECANNOT_DELETE));<br/>    DeleteRef &#123; self: ref.self &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_extend_ref"></a>

## Function `generate_extend_ref`

Generates the ExtendRef, which can be used to add new events and resources to the object.


<pre><code>public fun generate_extend_ref(ref: &amp;object::ConstructorRef): object::ExtendRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_extend_ref(ref: &amp;ConstructorRef): ExtendRef &#123;<br/>    ExtendRef &#123; self: ref.self &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Generates the TransferRef, which can be used to manage object transfers.


<pre><code>public fun generate_transfer_ref(ref: &amp;object::ConstructorRef): object::TransferRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_transfer_ref(ref: &amp;ConstructorRef): TransferRef &#123;<br/>    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));<br/>    TransferRef &#123; self: ref.self &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_derive_ref"></a>

## Function `generate_derive_ref`

Generates the DeriveRef, which can be used to create determnistic derived objects from the current object.


<pre><code>public fun generate_derive_ref(ref: &amp;object::ConstructorRef): object::DeriveRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_derive_ref(ref: &amp;ConstructorRef): DeriveRef &#123;<br/>    DeriveRef &#123; self: ref.self &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_signer"></a>

## Function `generate_signer`

Create a signer for the ConstructorRef


<pre><code>public fun generate_signer(ref: &amp;object::ConstructorRef): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_signer(ref: &amp;ConstructorRef): signer &#123;<br/>    create_signer(ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_address_from_constructor_ref"></a>

## Function `address_from_constructor_ref`

Returns the address associated with the constructor


<pre><code>public fun address_from_constructor_ref(ref: &amp;object::ConstructorRef): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_constructor_ref(ref: &amp;ConstructorRef): address &#123;<br/>    ref.self<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_object_from_constructor_ref"></a>

## Function `object_from_constructor_ref`

Returns an Object&lt;T&gt; from within a ConstructorRef


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;object::ConstructorRef): object::Object&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;ConstructorRef): Object&lt;T&gt; &#123;<br/>    address_to_object&lt;T&gt;(ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_can_generate_delete_ref"></a>

## Function `can_generate_delete_ref`

Returns whether or not the ConstructorRef can be used to create DeleteRef


<pre><code>public fun can_generate_delete_ref(ref: &amp;object::ConstructorRef): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_generate_delete_ref(ref: &amp;ConstructorRef): bool &#123;<br/>    ref.can_delete<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_create_guid"></a>

## Function `create_guid`

Create a guid for the object, typically used for events


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID acquires ObjectCore &#123;<br/>    let addr &#61; signer::address_of(object);<br/>    let object_data &#61; borrow_global_mut&lt;ObjectCore&gt;(addr);<br/>    guid::create(addr, &amp;mut object_data.guid_creation_num)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_new_event_handle"></a>

## Function `new_event_handle`

Generate a new event handle.


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(object: &amp;signer): event::EventHandle&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_event_handle&lt;T: drop &#43; store&gt;(<br/>    object: &amp;signer,<br/>): event::EventHandle&lt;T&gt; acquires ObjectCore &#123;<br/>    event::new_event_handle(create_guid(object))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_address_from_delete_ref"></a>

## Function `address_from_delete_ref`

Returns the address associated with the constructor


<pre><code>public fun address_from_delete_ref(ref: &amp;object::DeleteRef): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_delete_ref(ref: &amp;DeleteRef): address &#123;<br/>    ref.self<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_object_from_delete_ref"></a>

## Function `object_from_delete_ref`

Returns an Object&lt;T&gt; from within a DeleteRef.


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;object::DeleteRef): object::Object&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;DeleteRef): Object&lt;T&gt; &#123;<br/>    address_to_object&lt;T&gt;(ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_delete"></a>

## Function `delete`

Removes from the specified Object from global storage.


<pre><code>public fun delete(ref: object::DeleteRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delete(ref: DeleteRef) acquires Untransferable, ObjectCore &#123;<br/>    let object_core &#61; move_from&lt;ObjectCore&gt;(ref.self);<br/>    let ObjectCore &#123;<br/>        guid_creation_num: _,<br/>        owner: _,<br/>        allow_ungated_transfer: _,<br/>        transfer_events,<br/>    &#125; &#61; object_core;<br/><br/>    if (exists&lt;Untransferable&gt;(ref.self)) &#123;<br/>      let Untransferable &#123;&#125; &#61; move_from&lt;Untransferable&gt;(ref.self);<br/>    &#125;;<br/><br/>    event::destroy_handle(transfer_events);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_signer_for_extending"></a>

## Function `generate_signer_for_extending`

Create a signer for the ExtendRef


<pre><code>public fun generate_signer_for_extending(ref: &amp;object::ExtendRef): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_signer_for_extending(ref: &amp;ExtendRef): signer &#123;<br/>    create_signer(ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_address_from_extend_ref"></a>

## Function `address_from_extend_ref`

Returns an address from within a ExtendRef.


<pre><code>public fun address_from_extend_ref(ref: &amp;object::ExtendRef): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_extend_ref(ref: &amp;ExtendRef): address &#123;<br/>    ref.self<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_disable_ungated_transfer"></a>

## Function `disable_ungated_transfer`

Disable direct transfer, transfers can only be triggered via a TransferRef


<pre><code>public fun disable_ungated_transfer(ref: &amp;object::TransferRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun disable_ungated_transfer(ref: &amp;TransferRef) acquires ObjectCore &#123;<br/>    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);<br/>    object.allow_ungated_transfer &#61; false;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_set_untransferable"></a>

## Function `set_untransferable`

Prevent moving of the object


<pre><code>public fun set_untransferable(ref: &amp;object::ConstructorRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_untransferable(ref: &amp;ConstructorRef) acquires ObjectCore &#123;<br/>    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);<br/>    object.allow_ungated_transfer &#61; false;<br/>    let object_signer &#61; generate_signer(ref);<br/>    move_to(&amp;object_signer, Untransferable &#123;&#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_enable_ungated_transfer"></a>

## Function `enable_ungated_transfer`

Enable direct transfer.


<pre><code>public fun enable_ungated_transfer(ref: &amp;object::TransferRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun enable_ungated_transfer(ref: &amp;TransferRef) acquires ObjectCore &#123;<br/>    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));<br/>    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);<br/>    object.allow_ungated_transfer &#61; true;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_generate_linear_transfer_ref"></a>

## Function `generate_linear_transfer_ref`

Create a LinearTransferRef for a one&#45;time transfer. This requires that the owner at the<br/> time of generation is the owner at the time of transferring.


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;object::TransferRef): object::LinearTransferRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;TransferRef): LinearTransferRef acquires ObjectCore &#123;<br/>    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));<br/>    let owner &#61; owner(Object&lt;ObjectCore&gt; &#123; inner: ref.self &#125;);<br/>    LinearTransferRef &#123;<br/>        self: ref.self,<br/>        owner,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer to the destination address using a LinearTransferRef.


<pre><code>public fun transfer_with_ref(ref: object::LinearTransferRef, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_with_ref(ref: LinearTransferRef, to: address) acquires ObjectCore &#123;<br/>    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));<br/>    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);<br/>    assert!(<br/>        object.owner &#61;&#61; ref.owner,<br/>        error::permission_denied(ENOT_OBJECT_OWNER),<br/>    );<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            Transfer &#123;<br/>                object: ref.self,<br/>                from: object.owner,<br/>                to,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut object.transfer_events,<br/>        TransferEvent &#123;<br/>            object: ref.self,<br/>            from: object.owner,<br/>            to,<br/>        &#125;,<br/>    );<br/>    object.owner &#61; to;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer_call"></a>

## Function `transfer_call`

Entry function that can be used to transfer, if allow_ungated_transfer is set true.


<pre><code>public entry fun transfer_call(owner: &amp;signer, object: address, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_call(<br/>    owner: &amp;signer,<br/>    object: address,<br/>    to: address,<br/>) acquires ObjectCore &#123;<br/>    transfer_raw(owner, object, to)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer"></a>

## Function `transfer`

Transfers ownership of the object (and all associated resources) at the specified address<br/> for Object&lt;T&gt; to the &quot;to&quot; address.


<pre><code>public entry fun transfer&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(<br/>    owner: &amp;signer,<br/>    object: Object&lt;T&gt;,<br/>    to: address,<br/>) acquires ObjectCore &#123;<br/>    transfer_raw(owner, object.inner, to)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer_raw"></a>

## Function `transfer_raw`

Attempts to transfer using addresses only. Transfers the given object if<br/> allow_ungated_transfer is set true. Note, that this allows the owner of a nested object to<br/> transfer that object, so long as allow_ungated_transfer is enabled at each stage in the<br/> hierarchy.


<pre><code>public fun transfer_raw(owner: &amp;signer, object: address, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_raw(<br/>    owner: &amp;signer,<br/>    object: address,<br/>    to: address,<br/>) acquires ObjectCore &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    verify_ungated_and_descendant(owner_address, object);<br/>    transfer_raw_inner(object, to);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer_raw_inner"></a>

## Function `transfer_raw_inner`



<pre><code>fun transfer_raw_inner(object: address, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun transfer_raw_inner(object: address, to: address) acquires ObjectCore &#123;<br/>    let object_core &#61; borrow_global_mut&lt;ObjectCore&gt;(object);<br/>    if (object_core.owner !&#61; to) &#123;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Transfer &#123;<br/>                    object,<br/>                    from: object_core.owner,<br/>                    to,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut object_core.transfer_events,<br/>            TransferEvent &#123;<br/>                object,<br/>                from: object_core.owner,<br/>                to,<br/>            &#125;,<br/>        );<br/>        object_core.owner &#61; to;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_transfer_to_object"></a>

## Function `transfer_to_object`

Transfer the given object to another object. See <code>transfer</code> for more information.


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(owner: &amp;signer, object: object::Object&lt;O&gt;, to: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(<br/>    owner: &amp;signer,<br/>    object: Object&lt;O&gt;,<br/>    to: Object&lt;T&gt;,<br/>) acquires ObjectCore &#123;<br/>    transfer(owner, object, to.inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_verify_ungated_and_descendant"></a>

## Function `verify_ungated_and_descendant`

This checks that the destination address is eventually owned by the owner and that each<br/> object between the two allows for ungated transfers. Note, this is limited to a depth of 8<br/> objects may have cyclic dependencies.


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address) acquires ObjectCore &#123;<br/>    let current_address &#61; destination;<br/>    assert!(<br/>        exists&lt;ObjectCore&gt;(current_address),<br/>        error::not_found(EOBJECT_DOES_NOT_EXIST),<br/>    );<br/><br/>    let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);<br/>    assert!(<br/>        object.allow_ungated_transfer,<br/>        error::permission_denied(ENO_UNGATED_TRANSFERS),<br/>    );<br/><br/>    let current_address &#61; object.owner;<br/>    let count &#61; 0;<br/>    while (owner !&#61; current_address) &#123;<br/>        count &#61; count &#43; 1;<br/>        if (std::features::max_object_nesting_check_enabled()) &#123;<br/>            assert!(count &lt; MAXIMUM_OBJECT_NESTING, error::out_of_range(EMAXIMUM_NESTING))<br/>        &#125;;<br/>        // At this point, the first object exists and so the more likely case is that the<br/>        // object&apos;s owner is not an object. So we return a more sensible error.<br/>        assert!(<br/>            exists&lt;ObjectCore&gt;(current_address),<br/>            error::permission_denied(ENOT_OBJECT_OWNER),<br/>        );<br/>        let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);<br/>        assert!(<br/>            object.allow_ungated_transfer,<br/>            error::permission_denied(ENO_UNGATED_TRANSFERS),<br/>        );<br/>        current_address &#61; object.owner;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_burn"></a>

## Function `burn`

Forcefully transfer an unwanted object to BURN_ADDRESS, ignoring whether ungated_transfer is allowed.<br/> This only works for objects directly owned and for simplicity does not apply to indirectly owned objects.<br/> Original owners can reclaim burnt objects any time in the future by calling unburn.


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: Object&lt;T&gt;) acquires ObjectCore &#123;<br/>    let original_owner &#61; signer::address_of(owner);<br/>    assert!(is_owner(object, original_owner), error::permission_denied(ENOT_OBJECT_OWNER));<br/>    let object_addr &#61; object.inner;<br/>    move_to(&amp;create_signer(object_addr), TombStone &#123; original_owner &#125;);<br/>    transfer_raw_inner(object_addr, BURN_ADDRESS);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_unburn"></a>

## Function `unburn`

Allow origin owners to reclaim any objects they previous burnt.


<pre><code>public entry fun unburn&lt;T: key&gt;(original_owner: &amp;signer, object: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unburn&lt;T: key&gt;(<br/>    original_owner: &amp;signer,<br/>    object: Object&lt;T&gt;,<br/>) acquires TombStone, ObjectCore &#123;<br/>    let object_addr &#61; object.inner;<br/>    assert!(exists&lt;TombStone&gt;(object_addr), error::invalid_argument(EOBJECT_NOT_BURNT));<br/><br/>    let TombStone &#123; original_owner: original_owner_addr &#125; &#61; move_from&lt;TombStone&gt;(object_addr);<br/>    assert!(original_owner_addr &#61;&#61; signer::address_of(original_owner), error::permission_denied(ENOT_OBJECT_OWNER));<br/>    transfer_raw_inner(object_addr, original_owner_addr);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Accessors<br/> Return true if ungated transfer is allowed.


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: Object&lt;T&gt;): bool acquires ObjectCore &#123;<br/>    assert!(<br/>        exists&lt;ObjectCore&gt;(object.inner),<br/>        error::not_found(EOBJECT_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global&lt;ObjectCore&gt;(object.inner).allow_ungated_transfer<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_owner"></a>

## Function `owner`

Return the current owner.


<pre><code>public fun owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owner&lt;T: key&gt;(object: Object&lt;T&gt;): address acquires ObjectCore &#123;<br/>    assert!(<br/>        exists&lt;ObjectCore&gt;(object.inner),<br/>        error::not_found(EOBJECT_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global&lt;ObjectCore&gt;(object.inner).owner<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_is_owner"></a>

## Function `is_owner`

Return true if the provided address is the current owner.


<pre><code>public fun is_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_owner&lt;T: key&gt;(object: Object&lt;T&gt;, owner: address): bool acquires ObjectCore &#123;<br/>    owner(object) &#61;&#61; owner<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_owns"></a>

## Function `owns`

Return true if the provided address has indirect or direct ownership of the provided object.


<pre><code>public fun owns&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owns&lt;T: key&gt;(object: Object&lt;T&gt;, owner: address): bool acquires ObjectCore &#123;<br/>    let current_address &#61; object_address(&amp;object);<br/>    if (current_address &#61;&#61; owner) &#123;<br/>        return true<br/>    &#125;;<br/><br/>    assert!(<br/>        exists&lt;ObjectCore&gt;(current_address),<br/>        error::not_found(EOBJECT_DOES_NOT_EXIST),<br/>    );<br/><br/>    let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);<br/>    let current_address &#61; object.owner;<br/><br/>    let count &#61; 0;<br/>    while (owner !&#61; current_address) &#123;<br/>        count &#61; count &#43; 1;<br/>        if (std::features::max_object_nesting_check_enabled()) &#123;<br/>            assert!(count &lt; MAXIMUM_OBJECT_NESTING, error::out_of_range(EMAXIMUM_NESTING))<br/>        &#125;;<br/>        if (!exists&lt;ObjectCore&gt;(current_address)) &#123;<br/>            return false<br/>        &#125;;<br/><br/>        let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);<br/>        current_address &#61; object.owner;<br/>    &#125;;<br/>    true<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_root_owner"></a>

## Function `root_owner`

Returns the root owner of an object. As objects support nested ownership, it can be useful<br/> to determine the identity of the starting point of ownership.


<pre><code>public fun root_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun root_owner&lt;T: key&gt;(object: Object&lt;T&gt;): address acquires ObjectCore &#123;<br/>    let obj_owner &#61; owner(object);<br/>    while (is_object(obj_owner)) &#123;<br/>        obj_owner &#61; owner(address_to_object&lt;ObjectCore&gt;(obj_owner));<br/>    &#125;;<br/>    obj_owner<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;It&apos;s not possible to create an object twice on the same address.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The create_object_internal function includes an assertion to ensure that the object being created does not already exist at the specified address.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;create_object_internal&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;Only its owner may transfer an object.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The transfer function mandates that the transaction be signed by the owner&apos;s address, ensuring that only the rightful owner may initiate the object transfer.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if anyone other than the owner attempts to transfer.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The indirect owner of an object may transfer the object.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The owns function evaluates to true when the given address possesses either direct or indirect ownership of the specified object.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if address transferring is not indirect owner.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;Objects may never change the address which houses them.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;After creating an object, transfers to another owner may occur. However, the address which stores the object may not be changed.&lt;/td&gt;<br/>&lt;td&gt;This is implied by &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;If an ungated transfer is disabled on an object in an indirect ownership chain, a transfer should not occur.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Calling disable_ungated_transfer disables direct transfer, and only TransferRef may trigger transfers. The transfer_with_ref function is called.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;transfer_with_ref&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;Object addresses must not overlap with other addresses in different domains.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The current addressing scheme with suffixes does not conflict with any existing addresses, such as resource accounts. The GUID space is explicitly separated to ensure this doesn&apos;t happen.&lt;/td&gt;<br/>&lt;td&gt;This is true by construction if one correctly ensures the usage of INIT_GUID_CREATION_NUM during the creation of GUID.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma aborts_if_is_strict;<br/></code></pre>




<a id="0x1_object_spec_exists_at"></a>


<pre><code>fun spec_exists_at&lt;T: key&gt;(object: address): bool;<br/></code></pre>



<a id="@Specification_1_address_to_object"></a>

### Function `address_to_object`


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): object::Object&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object);<br/>aborts_if !spec_exists_at&lt;T&gt;(object);<br/>ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: object &#125;;<br/></code></pre>



<a id="@Specification_1_create_object_address"></a>

### Function `create_object_address`


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma aborts_if_is_strict &#61; false;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_create_object_address(source, seed);<br/></code></pre>




<a id="0x1_object_spec_create_user_derived_object_address_impl"></a>


<pre><code>fun spec_create_user_derived_object_address_impl(source: address, derive_from: address): address;<br/></code></pre>



<a id="@Specification_1_create_user_derived_object_address_impl"></a>

### Function `create_user_derived_object_address_impl`


<pre><code>fun create_user_derived_object_address_impl(source: address, derive_from: address): address<br/></code></pre>




<pre><code>pragma opaque;<br/>ensures [abstract] result &#61;&#61; spec_create_user_derived_object_address_impl(source, derive_from);<br/></code></pre>



<a id="@Specification_1_create_user_derived_object_address"></a>

### Function `create_user_derived_object_address`


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma aborts_if_is_strict &#61; false;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_create_user_derived_object_address(source, derive_from);<br/></code></pre>



<a id="@Specification_1_create_guid_object_address"></a>

### Function `create_guid_object_address`


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma aborts_if_is_strict &#61; false;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_create_guid_object_address(source, creation_num);<br/></code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code>fun exists_at&lt;T: key&gt;(object: address): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>ensures [abstract] result &#61;&#61; spec_exists_at&lt;T&gt;(object);<br/></code></pre>



<a id="@Specification_1_object_address"></a>

### Function `object_address`


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;): address<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; object.inner;<br/></code></pre>



<a id="@Specification_1_convert"></a>

### Function `convert`


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: object::Object&lt;X&gt;): object::Object&lt;Y&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);<br/>aborts_if !spec_exists_at&lt;Y&gt;(object.inner);<br/>ensures result &#61;&#61; Object&lt;Y&gt; &#123; inner: object.inner &#125;;<br/></code></pre>



<a id="@Specification_1_create_named_object"></a>

### Function `create_named_object`


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): object::ConstructorRef<br/></code></pre>




<pre><code>let creator_address &#61; signer::address_of(creator);<br/>let obj_addr &#61; spec_create_object_address(creator_address, seed);<br/>aborts_if exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: creator_address,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: obj_addr,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: false &#125;;<br/></code></pre>



<a id="@Specification_1_create_user_derived_object"></a>

### Function `create_user_derived_object`


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;object::DeriveRef): object::ConstructorRef<br/></code></pre>




<pre><code>let obj_addr &#61; spec_create_user_derived_object_address(creator_address, derive_ref.self);<br/>aborts_if exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: creator_address,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: obj_addr,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: false &#125;;<br/></code></pre>



<a id="@Specification_1_create_object"></a>

### Function `create_object`


<pre><code>public fun create_object(owner_address: address): object::ConstructorRef<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let unique_address &#61; transaction_context::spec_generate_unique_address();<br/>aborts_if exists&lt;ObjectCore&gt;(unique_address);<br/>ensures exists&lt;ObjectCore&gt;(unique_address);<br/>ensures global&lt;ObjectCore&gt;(unique_address) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: owner_address,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: unique_address,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: unique_address, can_delete: true &#125;;<br/></code></pre>



<a id="@Specification_1_create_sticky_object"></a>

### Function `create_sticky_object`


<pre><code>public fun create_sticky_object(owner_address: address): object::ConstructorRef<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let unique_address &#61; transaction_context::spec_generate_unique_address();<br/>aborts_if exists&lt;ObjectCore&gt;(unique_address);<br/>ensures exists&lt;ObjectCore&gt;(unique_address);<br/>ensures global&lt;ObjectCore&gt;(unique_address) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: owner_address,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: unique_address,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: unique_address, can_delete: false &#125;;<br/></code></pre>



<a id="@Specification_1_create_sticky_object_at_address"></a>

### Function `create_sticky_object_at_address`


<pre><code>public(friend) fun create_sticky_object_at_address(owner_address: address, object_address: address): object::ConstructorRef<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_create_object_from_account"></a>

### Function `create_object_from_account`


<pre><code>&#35;[deprecated]<br/>public fun create_object_from_account(creator: &amp;signer): object::ConstructorRef<br/></code></pre>




<pre><code>aborts_if !exists&lt;account::Account&gt;(signer::address_of(creator));<br/>let object_data &#61; global&lt;account::Account&gt;(signer::address_of(creator));<br/>aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;<br/>aborts_if object_data.guid_creation_num &#43; 1 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>let creation_num &#61; object_data.guid_creation_num;<br/>let addr &#61; signer::address_of(creator);<br/>let guid &#61; guid::GUID &#123;<br/>    id: guid::ID &#123;<br/>        creation_num,<br/>        addr,<br/>    &#125;<br/>&#125;;<br/>let bytes_spec &#61; bcs::to_bytes(guid);<br/>let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));<br/>let hash_bytes &#61; hash::sha3_256(bytes);<br/>let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);<br/>aborts_if exists&lt;ObjectCore&gt;(obj_addr);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);<br/>ensures global&lt;account::Account&gt;(addr).guid_creation_num &#61;&#61; old(<br/>    global&lt;account::Account&gt;(addr)<br/>).guid_creation_num &#43; 1;<br/>ensures exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: addr,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: obj_addr,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;<br/></code></pre>



<a id="@Specification_1_create_object_from_object"></a>

### Function `create_object_from_object`


<pre><code>&#35;[deprecated]<br/>public fun create_object_from_object(creator: &amp;signer): object::ConstructorRef<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(creator));<br/>let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(creator));<br/>aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;<br/>let creation_num &#61; object_data.guid_creation_num;<br/>let addr &#61; signer::address_of(creator);<br/>let guid &#61; guid::GUID &#123;<br/>    id: guid::ID &#123;<br/>        creation_num,<br/>        addr,<br/>    &#125;<br/>&#125;;<br/>let bytes_spec &#61; bcs::to_bytes(guid);<br/>let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));<br/>let hash_bytes &#61; hash::sha3_256(bytes);<br/>let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);<br/>aborts_if exists&lt;ObjectCore&gt;(obj_addr);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);<br/>ensures global&lt;ObjectCore&gt;(addr).guid_creation_num &#61;&#61; old(global&lt;ObjectCore&gt;(addr)).guid_creation_num &#43; 1;<br/>ensures exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: addr,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: obj_addr,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;<br/></code></pre>



<a id="@Specification_1_create_object_from_guid"></a>

### Function `create_object_from_guid`


<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): object::ConstructorRef<br/></code></pre>




<pre><code>let bytes_spec &#61; bcs::to_bytes(guid);<br/>let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));<br/>let hash_bytes &#61; hash::sha3_256(bytes);<br/>let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);<br/>aborts_if exists&lt;ObjectCore&gt;(obj_addr);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);<br/>ensures exists&lt;ObjectCore&gt;(obj_addr);<br/>ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;<br/>    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,<br/>    owner: creator_address,<br/>    allow_ungated_transfer: true,<br/>    transfer_events: event::EventHandle &#123;<br/>        counter: 0,<br/>        guid: guid::GUID &#123;<br/>            id: guid::ID &#123;<br/>                creation_num: INIT_GUID_CREATION_NUM,<br/>                addr: obj_addr,<br/>            &#125;<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;<br/></code></pre>



<a id="@Specification_1_create_object_internal"></a>

### Function `create_object_internal`


<pre><code>fun create_object_internal(creator_address: address, object: address, can_delete: bool): object::ConstructorRef<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if exists&lt;ObjectCore&gt;(object);<br/>ensures exists&lt;ObjectCore&gt;(object);<br/>ensures global&lt;ObjectCore&gt;(object).guid_creation_num &#61;&#61; INIT_GUID_CREATION_NUM &#43; 1;<br/>ensures result &#61;&#61; ConstructorRef &#123; self: object, can_delete &#125;;<br/></code></pre>



<a id="@Specification_1_generate_delete_ref"></a>

### Function `generate_delete_ref`


<pre><code>public fun generate_delete_ref(ref: &amp;object::ConstructorRef): object::DeleteRef<br/></code></pre>




<pre><code>aborts_if !ref.can_delete;<br/>ensures result &#61;&#61; DeleteRef &#123; self: ref.self &#125;;<br/></code></pre>



<a id="@Specification_1_generate_transfer_ref"></a>

### Function `generate_transfer_ref`


<pre><code>public fun generate_transfer_ref(ref: &amp;object::ConstructorRef): object::TransferRef<br/></code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);<br/>ensures result &#61;&#61; TransferRef &#123;<br/>    self: ref.self,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_object_from_constructor_ref"></a>

### Function `object_from_constructor_ref`


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;object::ConstructorRef): object::Object&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>aborts_if !spec_exists_at&lt;T&gt;(ref.self);<br/>ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: ref.self &#125;;<br/></code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(object));<br/>let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(object));<br/>aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;<br/>ensures result &#61;&#61; guid::GUID &#123;<br/>    id: guid::ID &#123;<br/>        creation_num: object_data.guid_creation_num,<br/>        addr: signer::address_of(object)<br/>    &#125;<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(object: &amp;signer): event::EventHandle&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(object));<br/>let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(object));<br/>aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;<br/>let guid &#61; guid::GUID &#123;<br/>    id: guid::ID &#123;<br/>        creation_num: object_data.guid_creation_num,<br/>        addr: signer::address_of(object)<br/>    &#125;<br/>&#125;;<br/>ensures result &#61;&#61; event::EventHandle&lt;T&gt; &#123;<br/>    counter: 0,<br/>    guid,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_object_from_delete_ref"></a>

### Function `object_from_delete_ref`


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;object::DeleteRef): object::Object&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>aborts_if !spec_exists_at&lt;T&gt;(ref.self);<br/>ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: ref.self &#125;;<br/></code></pre>



<a id="@Specification_1_delete"></a>

### Function `delete`


<pre><code>public fun delete(ref: object::DeleteRef)<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>ensures !exists&lt;ObjectCore&gt;(ref.self);<br/></code></pre>



<a id="@Specification_1_disable_ungated_transfer"></a>

### Function `disable_ungated_transfer`


<pre><code>public fun disable_ungated_transfer(ref: &amp;object::TransferRef)<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; false;<br/></code></pre>



<a id="@Specification_1_set_untransferable"></a>

### Function `set_untransferable`


<pre><code>public fun set_untransferable(ref: &amp;object::ConstructorRef)<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>aborts_if exists&lt;Untransferable&gt;(ref.self);<br/>ensures exists&lt;Untransferable&gt;(ref.self);<br/>ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; false;<br/></code></pre>



<a id="@Specification_1_enable_ungated_transfer"></a>

### Function `enable_ungated_transfer`


<pre><code>public fun enable_ungated_transfer(ref: &amp;object::TransferRef)<br/></code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);<br/>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; true;<br/></code></pre>



<a id="@Specification_1_generate_linear_transfer_ref"></a>

### Function `generate_linear_transfer_ref`


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;object::TransferRef): object::LinearTransferRef<br/></code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);<br/>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>let owner &#61; global&lt;ObjectCore&gt;(ref.self).owner;<br/>ensures result &#61;&#61; LinearTransferRef &#123;<br/>    self: ref.self,<br/>    owner,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_transfer_with_ref"></a>

### Function `transfer_with_ref`


<pre><code>public fun transfer_with_ref(ref: object::LinearTransferRef, to: address)<br/></code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);<br/>let object &#61; global&lt;ObjectCore&gt;(ref.self);<br/>aborts_if !exists&lt;ObjectCore&gt;(ref.self);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if object.owner !&#61; ref.owner;<br/>ensures global&lt;ObjectCore&gt;(ref.self).owner &#61;&#61; to;<br/></code></pre>



<a id="@Specification_1_transfer_call"></a>

### Function `transfer_call`


<pre><code>public entry fun transfer_call(owner: &amp;signer, object: address, to: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let owner_address &#61; signer::address_of(owner);<br/>aborts_if !exists&lt;ObjectCore&gt;(object);<br/>aborts_if !global&lt;ObjectCore&gt;(object).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;, to: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let owner_address &#61; signer::address_of(owner);<br/>let object_address &#61; object.inner;<br/>aborts_if !exists&lt;ObjectCore&gt;(object_address);<br/>aborts_if !global&lt;ObjectCore&gt;(object_address).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_transfer_raw"></a>

### Function `transfer_raw`


<pre><code>public fun transfer_raw(owner: &amp;signer, object: address, to: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let owner_address &#61; signer::address_of(owner);<br/>aborts_if !exists&lt;ObjectCore&gt;(object);<br/>aborts_if !global&lt;ObjectCore&gt;(object).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_transfer_to_object"></a>

### Function `transfer_to_object`


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(owner: &amp;signer, object: object::Object&lt;O&gt;, to: object::Object&lt;T&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let owner_address &#61; signer::address_of(owner);<br/>let object_address &#61; object.inner;<br/>aborts_if !exists&lt;ObjectCore&gt;(object_address);<br/>aborts_if !global&lt;ObjectCore&gt;(object_address).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_verify_ungated_and_descendant"></a>

### Function `verify_ungated_and_descendant`


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>pragma unroll &#61; MAXIMUM_OBJECT_NESTING;<br/>aborts_if !exists&lt;ObjectCore&gt;(destination);<br/>aborts_if !global&lt;ObjectCore&gt;(destination).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let object_address &#61; object.inner;<br/>aborts_if !exists&lt;ObjectCore&gt;(object_address);<br/>aborts_if owner(object) !&#61; signer::address_of(owner);<br/>aborts_if is_burnt(object);<br/></code></pre>



<a id="@Specification_1_unburn"></a>

### Function `unburn`


<pre><code>public entry fun unburn&lt;T: key&gt;(original_owner: &amp;signer, object: object::Object&lt;T&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let object_address &#61; object.inner;<br/>aborts_if !exists&lt;ObjectCore&gt;(object_address);<br/>aborts_if !is_burnt(object);<br/>let tomb_stone &#61; borrow_global&lt;TombStone&gt;(object_address);<br/>aborts_if tomb_stone.original_owner !&#61; signer::address_of(original_owner);<br/></code></pre>



<a id="@Specification_1_ungated_transfer_allowed"></a>

### Function `ungated_transfer_allowed`


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);<br/>ensures result &#61;&#61; global&lt;ObjectCore&gt;(object.inner).allow_ungated_transfer;<br/></code></pre>



<a id="@Specification_1_owner"></a>

### Function `owner`


<pre><code>public fun owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);<br/>ensures result &#61;&#61; global&lt;ObjectCore&gt;(object.inner).owner;<br/></code></pre>



<a id="@Specification_1_is_owner"></a>

### Function `is_owner`


<pre><code>public fun is_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool<br/></code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);<br/>ensures result &#61;&#61; (global&lt;ObjectCore&gt;(object.inner).owner &#61;&#61; owner);<br/></code></pre>



<a id="@Specification_1_owns"></a>

### Function `owns`


<pre><code>public fun owns&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let current_address_0 &#61; object.inner;<br/>let object_0 &#61; global&lt;ObjectCore&gt;(current_address_0);<br/>let current_address &#61; object_0.owner;<br/>aborts_if object.inner !&#61; owner &amp;&amp; !exists&lt;ObjectCore&gt;(object.inner);<br/>ensures current_address_0 &#61;&#61; owner &#61;&#61;&gt; result &#61;&#61; true;<br/></code></pre>



<a id="@Specification_1_root_owner"></a>

### Function `root_owner`


<pre><code>public fun root_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/></code></pre>




<a id="0x1_object_spec_create_object_address"></a>


<pre><code>fun spec_create_object_address(source: address, seed: vector&lt;u8&gt;): address;<br/></code></pre>




<a id="0x1_object_spec_create_user_derived_object_address"></a>


<pre><code>fun spec_create_user_derived_object_address(source: address, derive_from: address): address;<br/></code></pre>




<a id="0x1_object_spec_create_guid_object_address"></a>


<pre><code>fun spec_create_guid_object_address(source: address, creation_num: u64): address;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
