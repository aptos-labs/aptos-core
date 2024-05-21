
<a id="0x1_object"></a>

# Module `0x1::object`

This defines the Move object model with the following properties:
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


<pre><code>use 0x1::account;
use 0x1::bcs;
use 0x1::create_signer;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::from_bcs;
use 0x1::guid;
use 0x1::hash;
use 0x1::signer;
use 0x1::transaction_context;
use 0x1::vector;
</code></pre>



<a id="0x1_object_ObjectCore"></a>

## Resource `ObjectCore`

The core of the object model that defines ownership, transferability, and events.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct ObjectCore has key
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
<code>owner: address</code>
</dt>
<dd>
 The address (object or account) that owns this object
</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>
 Object transferring is a common operation, this allows for disabling and enabling
 transfers bypassing the use of a TransferRef.
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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct TombStone has key
</code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct Untransferable has key
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

<a id="0x1_object_ObjectGroup"></a>

## Struct `ObjectGroup`

A shared resource group for storing object resources together in storage.


<pre><code>&#35;[resource_group(&#35;[scope &#61; global])]
struct ObjectGroup
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

<a id="0x1_object_Object"></a>

## Struct `Object`

A pointer to an object -- these can only provide guarantees based upon the underlying data
type, that is the validity of T existing at an address is something that cannot be verified
by any other module than the module that defined T. Similarly, the module that defines T
can remove it from storage at any point in time.


<pre><code>struct Object&lt;T&gt; has copy, drop, store
</code></pre>



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


<pre><code>struct ConstructorRef has drop
</code></pre>



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


<pre><code>struct DeleteRef has drop, store
</code></pre>



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


<pre><code>struct ExtendRef has drop, store
</code></pre>



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


<pre><code>struct TransferRef has drop, store
</code></pre>



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

Used to perform transfers. This locks transferring ability to a single time use bound to
the current owner.


<pre><code>struct LinearTransferRef has drop
</code></pre>



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


<pre><code>struct DeriveRef has drop, store
</code></pre>



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

Emitted whenever the object's owner field is changed.


<pre><code>struct TransferEvent has drop, store
</code></pre>



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

Emitted whenever the object's owner field is changed.


<pre><code>&#35;[event]
struct Transfer has drop, store
</code></pre>



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


<pre><code>const BURN_ADDRESS: address &#61; 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
</code></pre>



<a id="0x1_object_DERIVE_AUID_ADDRESS_SCHEME"></a>

generate_unique_address uses this for domain separation within its native implementation


<pre><code>const DERIVE_AUID_ADDRESS_SCHEME: u8 &#61; 251;
</code></pre>



<a id="0x1_object_ECANNOT_DELETE"></a>

The object does not allow for deletion


<pre><code>const ECANNOT_DELETE: u64 &#61; 5;
</code></pre>



<a id="0x1_object_EMAXIMUM_NESTING"></a>

Exceeds maximum nesting for an object transfer.


<pre><code>const EMAXIMUM_NESTING: u64 &#61; 6;
</code></pre>



<a id="0x1_object_ENOT_MOVABLE"></a>

Object is untransferable any operations that might result in a transfer are disallowed.


<pre><code>const ENOT_MOVABLE: u64 &#61; 9;
</code></pre>



<a id="0x1_object_ENOT_OBJECT_OWNER"></a>

The caller does not have ownership permissions


<pre><code>const ENOT_OBJECT_OWNER: u64 &#61; 4;
</code></pre>



<a id="0x1_object_ENO_UNGATED_TRANSFERS"></a>

The object does not have ungated transfers enabled


<pre><code>const ENO_UNGATED_TRANSFERS: u64 &#61; 3;
</code></pre>



<a id="0x1_object_EOBJECT_DOES_NOT_EXIST"></a>

An object does not exist at this address


<pre><code>const EOBJECT_DOES_NOT_EXIST: u64 &#61; 2;
</code></pre>



<a id="0x1_object_EOBJECT_EXISTS"></a>

An object already exists at this address


<pre><code>const EOBJECT_EXISTS: u64 &#61; 1;
</code></pre>



<a id="0x1_object_EOBJECT_NOT_BURNT"></a>

Cannot reclaim objects that weren't burnt.


<pre><code>const EOBJECT_NOT_BURNT: u64 &#61; 8;
</code></pre>



<a id="0x1_object_ERESOURCE_DOES_NOT_EXIST"></a>

The resource is not stored at the specified address.


<pre><code>const ERESOURCE_DOES_NOT_EXIST: u64 &#61; 7;
</code></pre>



<a id="0x1_object_INIT_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code>const INIT_GUID_CREATION_NUM: u64 &#61; 1125899906842624;
</code></pre>



<a id="0x1_object_MAXIMUM_OBJECT_NESTING"></a>

Maximum nesting from one object to another. That is objects can technically have infinte
nesting, but any checks such as transfer will only be evaluated this deep.


<pre><code>const MAXIMUM_OBJECT_NESTING: u8 &#61; 8;
</code></pre>



<a id="0x1_object_OBJECT_DERIVED_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> as derived from another object.
The object's address is generated as:
```
obj_addr = sha3_256(account addr | derived from object's address | 0xFC)
```

This 0xFC constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code>const OBJECT_DERIVED_SCHEME: u8 &#61; 252;
</code></pre>



<a id="0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> via a fresh GUID generated by the creator at
<code>source_addr</code>. The object's address is generated as:
```
obj_addr = sha3_256(guid | 0xFD)
```
where <code>guid &#61; account::create_guid(create_signer(source_addr))</code>

This 0xFD constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code>const OBJECT_FROM_GUID_ADDRESS_SCHEME: u8 &#61; 253;
</code></pre>



<a id="0x1_object_OBJECT_FROM_SEED_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> from the creator's <code>source_addr</code> and a <code>seed</code> as:
obj_addr = sha3_256(source_addr | seed | 0xFE).

This 0xFE constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code>const OBJECT_FROM_SEED_ADDRESS_SCHEME: u8 &#61; 254;
</code></pre>



<a id="0x1_object_is_untransferable"></a>

## Function `is_untransferable`



<pre><code>&#35;[view]
public fun is_untransferable&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_untransferable&lt;T: key&gt;(object: Object&lt;T&gt;): bool &#123;
    exists&lt;Untransferable&gt;(object.inner)
&#125;
</code></pre>



</details>

<a id="0x1_object_is_burnt"></a>

## Function `is_burnt`



<pre><code>&#35;[view]
public fun is_burnt&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_burnt&lt;T: key&gt;(object: Object&lt;T&gt;): bool &#123;
    exists&lt;TombStone&gt;(object.inner)
&#125;
</code></pre>



</details>

<a id="0x1_object_address_to_object"></a>

## Function `address_to_object`

Produces an ObjectId from the given address. This is not verified.


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): object::Object&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): Object&lt;T&gt; &#123;
    assert!(exists&lt;ObjectCore&gt;(object), error::not_found(EOBJECT_DOES_NOT_EXIST));
    assert!(exists_at&lt;T&gt;(object), error::not_found(ERESOURCE_DOES_NOT_EXIST));
    Object&lt;T&gt; &#123; inner: object &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_is_object"></a>

## Function `is_object`

Returns true if there exists an object or the remnants of an object.


<pre><code>public fun is_object(object: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_object(object: address): bool &#123;
    exists&lt;ObjectCore&gt;(object)
&#125;
</code></pre>



</details>

<a id="0x1_object_object_exists"></a>

## Function `object_exists`

Returns true if there exists an object with resource T.


<pre><code>public fun object_exists&lt;T: key&gt;(object: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_exists&lt;T: key&gt;(object: address): bool &#123;
    exists&lt;ObjectCore&gt;(object) &amp;&amp; exists_at&lt;T&gt;(object)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object_address"></a>

## Function `create_object_address`

Derives an object address from source material: sha3_256([creator address | seed | 0xFE]).


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address &#123;
    let bytes &#61; bcs::to_bytes(source);
    vector::append(&amp;mut bytes, seed);
    vector::push_back(&amp;mut bytes, OBJECT_FROM_SEED_ADDRESS_SCHEME);
    from_bcs::to_address(hash::sha3_256(bytes))
&#125;
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address_impl"></a>

## Function `create_user_derived_object_address_impl`



<pre><code>fun create_user_derived_object_address_impl(source: address, derive_from: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun create_user_derived_object_address_impl(source: address, derive_from: address): address;
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address"></a>

## Function `create_user_derived_object_address`

Derives an object address from the source address and an object: sha3_256([source | object addr | 0xFC]).


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address &#123;
    if (std::features::object_native_derived_address_enabled()) &#123;
        create_user_derived_object_address_impl(source, derive_from)
    &#125; else &#123;
        let bytes &#61; bcs::to_bytes(&amp;source);
        vector::append(&amp;mut bytes, bcs::to_bytes(&amp;derive_from));
        vector::push_back(&amp;mut bytes, OBJECT_DERIVED_SCHEME);
        from_bcs::to_address(hash::sha3_256(bytes))
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_create_guid_object_address"></a>

## Function `create_guid_object_address`

Derives an object from an Account GUID.


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address &#123;
    let id &#61; guid::create_id(source, creation_num);
    let bytes &#61; bcs::to_bytes(&amp;id);
    vector::push_back(&amp;mut bytes, OBJECT_FROM_GUID_ADDRESS_SCHEME);
    from_bcs::to_address(hash::sha3_256(bytes))
&#125;
</code></pre>



</details>

<a id="0x1_object_exists_at"></a>

## Function `exists_at`



<pre><code>fun exists_at&lt;T: key&gt;(object: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun exists_at&lt;T: key&gt;(object: address): bool;
</code></pre>



</details>

<a id="0x1_object_object_address"></a>

## Function `object_address`

Returns the address of within an ObjectId.


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;Object&lt;T&gt;): address &#123;
    object.inner
&#125;
</code></pre>



</details>

<a id="0x1_object_convert"></a>

## Function `convert`

Convert Object<X> to Object<Y>.


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: object::Object&lt;X&gt;): object::Object&lt;Y&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: Object&lt;X&gt;): Object&lt;Y&gt; &#123;
    address_to_object&lt;Y&gt;(object.inner)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_named_object"></a>

## Function `create_named_object`

Create a new named object and return the ConstructorRef. Named objects can be queried globally
by knowing the user generated seed used to create them. Named objects cannot be deleted.


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): ConstructorRef &#123;
    let creator_address &#61; signer::address_of(creator);
    let obj_addr &#61; create_object_address(&amp;creator_address, seed);
    create_object_internal(creator_address, obj_addr, false)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object"></a>

## Function `create_user_derived_object`

Create a new object whose address is derived based on the creator account address and another object.
Derivde objects, similar to named objects, cannot be deleted.


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;object::DeriveRef): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;DeriveRef): ConstructorRef &#123;
    let obj_addr &#61; create_user_derived_object_address(creator_address, derive_ref.self);
    create_object_internal(creator_address, obj_addr, false)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object"></a>

## Function `create_object`

Create a new object by generating a random unique address based on transaction hash.
The unique address is computed sha3_256([transaction hash | auid counter | 0xFB]).
The created object is deletable as we can guarantee the same unique address can
never be regenerated with future txs.


<pre><code>public fun create_object(owner_address: address): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object(owner_address: address): ConstructorRef &#123;
    let unique_address &#61; transaction_context::generate_auid_address();
    create_object_internal(owner_address, unique_address, true)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_sticky_object"></a>

## Function `create_sticky_object`

Same as <code>create_object</code> except the object to be created will be undeletable.


<pre><code>public fun create_sticky_object(owner_address: address): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_sticky_object(owner_address: address): ConstructorRef &#123;
    let unique_address &#61; transaction_context::generate_auid_address();
    create_object_internal(owner_address, unique_address, false)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_sticky_object_at_address"></a>

## Function `create_sticky_object_at_address`

Create a sticky object at a specific address. Only used by aptos_framework::coin.


<pre><code>public(friend) fun create_sticky_object_at_address(owner_address: address, object_address: address): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_sticky_object_at_address(
    owner_address: address,
    object_address: address,
): ConstructorRef &#123;
    create_object_internal(owner_address, object_address, false)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object_from_account"></a>

## Function `create_object_from_account`

Use <code>create_object</code> instead.
Create a new object from a GUID generated by an account.
As the GUID creation internally increments a counter, two transactions that executes
<code>create_object_from_account</code> function for the same creator run sequentially.
Therefore, using <code>create_object</code> method for creating objects is preferrable as it
doesn't have the same bottlenecks.


<pre><code>&#35;[deprecated]
public fun create_object_from_account(creator: &amp;signer): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_from_account(creator: &amp;signer): ConstructorRef &#123;
    let guid &#61; account::create_guid(creator);
    create_object_from_guid(signer::address_of(creator), guid)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object_from_object"></a>

## Function `create_object_from_object`

Use <code>create_object</code> instead.
Create a new object from a GUID generated by an object.
As the GUID creation internally increments a counter, two transactions that executes
<code>create_object_from_object</code> function for the same creator run sequentially.
Therefore, using <code>create_object</code> method for creating objects is preferrable as it
doesn't have the same bottlenecks.


<pre><code>&#35;[deprecated]
public fun create_object_from_object(creator: &amp;signer): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_object_from_object(creator: &amp;signer): ConstructorRef acquires ObjectCore &#123;
    let guid &#61; create_guid(creator);
    create_object_from_guid(signer::address_of(creator), guid)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object_from_guid"></a>

## Function `create_object_from_guid`



<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef &#123;
    let bytes &#61; bcs::to_bytes(&amp;guid);
    vector::push_back(&amp;mut bytes, OBJECT_FROM_GUID_ADDRESS_SCHEME);
    let obj_addr &#61; from_bcs::to_address(hash::sha3_256(bytes));
    create_object_internal(creator_address, obj_addr, true)
&#125;
</code></pre>



</details>

<a id="0x1_object_create_object_internal"></a>

## Function `create_object_internal`



<pre><code>fun create_object_internal(creator_address: address, object: address, can_delete: bool): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_object_internal(
    creator_address: address,
    object: address,
    can_delete: bool,
): ConstructorRef &#123;
    assert!(!exists&lt;ObjectCore&gt;(object), error::already_exists(EOBJECT_EXISTS));

    let object_signer &#61; create_signer(object);
    let guid_creation_num &#61; INIT_GUID_CREATION_NUM;
    let transfer_events_guid &#61; guid::create(object, &amp;mut guid_creation_num);

    move_to(
        &amp;object_signer,
        ObjectCore &#123;
            guid_creation_num,
            owner: creator_address,
            allow_ungated_transfer: true,
            transfer_events: event::new_event_handle(transfer_events_guid),
        &#125;,
    );
    ConstructorRef &#123; self: object, can_delete &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_delete_ref"></a>

## Function `generate_delete_ref`

Generates the DeleteRef, which can be used to remove ObjectCore from global storage.


<pre><code>public fun generate_delete_ref(ref: &amp;object::ConstructorRef): object::DeleteRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_delete_ref(ref: &amp;ConstructorRef): DeleteRef &#123;
    assert!(ref.can_delete, error::permission_denied(ECANNOT_DELETE));
    DeleteRef &#123; self: ref.self &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_extend_ref"></a>

## Function `generate_extend_ref`

Generates the ExtendRef, which can be used to add new events and resources to the object.


<pre><code>public fun generate_extend_ref(ref: &amp;object::ConstructorRef): object::ExtendRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_extend_ref(ref: &amp;ConstructorRef): ExtendRef &#123;
    ExtendRef &#123; self: ref.self &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Generates the TransferRef, which can be used to manage object transfers.


<pre><code>public fun generate_transfer_ref(ref: &amp;object::ConstructorRef): object::TransferRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_transfer_ref(ref: &amp;ConstructorRef): TransferRef &#123;
    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));
    TransferRef &#123; self: ref.self &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_derive_ref"></a>

## Function `generate_derive_ref`

Generates the DeriveRef, which can be used to create determnistic derived objects from the current object.


<pre><code>public fun generate_derive_ref(ref: &amp;object::ConstructorRef): object::DeriveRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_derive_ref(ref: &amp;ConstructorRef): DeriveRef &#123;
    DeriveRef &#123; self: ref.self &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_signer"></a>

## Function `generate_signer`

Create a signer for the ConstructorRef


<pre><code>public fun generate_signer(ref: &amp;object::ConstructorRef): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_signer(ref: &amp;ConstructorRef): signer &#123;
    create_signer(ref.self)
&#125;
</code></pre>



</details>

<a id="0x1_object_address_from_constructor_ref"></a>

## Function `address_from_constructor_ref`

Returns the address associated with the constructor


<pre><code>public fun address_from_constructor_ref(ref: &amp;object::ConstructorRef): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_constructor_ref(ref: &amp;ConstructorRef): address &#123;
    ref.self
&#125;
</code></pre>



</details>

<a id="0x1_object_object_from_constructor_ref"></a>

## Function `object_from_constructor_ref`

Returns an Object<T> from within a ConstructorRef


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;object::ConstructorRef): object::Object&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;ConstructorRef): Object&lt;T&gt; &#123;
    address_to_object&lt;T&gt;(ref.self)
&#125;
</code></pre>



</details>

<a id="0x1_object_can_generate_delete_ref"></a>

## Function `can_generate_delete_ref`

Returns whether or not the ConstructorRef can be used to create DeleteRef


<pre><code>public fun can_generate_delete_ref(ref: &amp;object::ConstructorRef): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_generate_delete_ref(ref: &amp;ConstructorRef): bool &#123;
    ref.can_delete
&#125;
</code></pre>



</details>

<a id="0x1_object_create_guid"></a>

## Function `create_guid`

Create a guid for the object, typically used for events


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID acquires ObjectCore &#123;
    let addr &#61; signer::address_of(object);
    let object_data &#61; borrow_global_mut&lt;ObjectCore&gt;(addr);
    guid::create(addr, &amp;mut object_data.guid_creation_num)
&#125;
</code></pre>



</details>

<a id="0x1_object_new_event_handle"></a>

## Function `new_event_handle`

Generate a new event handle.


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(object: &amp;signer): event::EventHandle&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_event_handle&lt;T: drop &#43; store&gt;(
    object: &amp;signer,
): event::EventHandle&lt;T&gt; acquires ObjectCore &#123;
    event::new_event_handle(create_guid(object))
&#125;
</code></pre>



</details>

<a id="0x1_object_address_from_delete_ref"></a>

## Function `address_from_delete_ref`

Returns the address associated with the constructor


<pre><code>public fun address_from_delete_ref(ref: &amp;object::DeleteRef): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_delete_ref(ref: &amp;DeleteRef): address &#123;
    ref.self
&#125;
</code></pre>



</details>

<a id="0x1_object_object_from_delete_ref"></a>

## Function `object_from_delete_ref`

Returns an Object<T> from within a DeleteRef.


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;object::DeleteRef): object::Object&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;DeleteRef): Object&lt;T&gt; &#123;
    address_to_object&lt;T&gt;(ref.self)
&#125;
</code></pre>



</details>

<a id="0x1_object_delete"></a>

## Function `delete`

Removes from the specified Object from global storage.


<pre><code>public fun delete(ref: object::DeleteRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delete(ref: DeleteRef) acquires Untransferable, ObjectCore &#123;
    let object_core &#61; move_from&lt;ObjectCore&gt;(ref.self);
    let ObjectCore &#123;
        guid_creation_num: _,
        owner: _,
        allow_ungated_transfer: _,
        transfer_events,
    &#125; &#61; object_core;

    if (exists&lt;Untransferable&gt;(ref.self)) &#123;
      let Untransferable &#123;&#125; &#61; move_from&lt;Untransferable&gt;(ref.self);
    &#125;;

    event::destroy_handle(transfer_events);
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_signer_for_extending"></a>

## Function `generate_signer_for_extending`

Create a signer for the ExtendRef


<pre><code>public fun generate_signer_for_extending(ref: &amp;object::ExtendRef): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_signer_for_extending(ref: &amp;ExtendRef): signer &#123;
    create_signer(ref.self)
&#125;
</code></pre>



</details>

<a id="0x1_object_address_from_extend_ref"></a>

## Function `address_from_extend_ref`

Returns an address from within a ExtendRef.


<pre><code>public fun address_from_extend_ref(ref: &amp;object::ExtendRef): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_extend_ref(ref: &amp;ExtendRef): address &#123;
    ref.self
&#125;
</code></pre>



</details>

<a id="0x1_object_disable_ungated_transfer"></a>

## Function `disable_ungated_transfer`

Disable direct transfer, transfers can only be triggered via a TransferRef


<pre><code>public fun disable_ungated_transfer(ref: &amp;object::TransferRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun disable_ungated_transfer(ref: &amp;TransferRef) acquires ObjectCore &#123;
    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);
    object.allow_ungated_transfer &#61; false;
&#125;
</code></pre>



</details>

<a id="0x1_object_set_untransferable"></a>

## Function `set_untransferable`

Prevent moving of the object


<pre><code>public fun set_untransferable(ref: &amp;object::ConstructorRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_untransferable(ref: &amp;ConstructorRef) acquires ObjectCore &#123;
    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);
    object.allow_ungated_transfer &#61; false;
    let object_signer &#61; generate_signer(ref);
    move_to(&amp;object_signer, Untransferable &#123;&#125;);
&#125;
</code></pre>



</details>

<a id="0x1_object_enable_ungated_transfer"></a>

## Function `enable_ungated_transfer`

Enable direct transfer.


<pre><code>public fun enable_ungated_transfer(ref: &amp;object::TransferRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun enable_ungated_transfer(ref: &amp;TransferRef) acquires ObjectCore &#123;
    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));
    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);
    object.allow_ungated_transfer &#61; true;
&#125;
</code></pre>



</details>

<a id="0x1_object_generate_linear_transfer_ref"></a>

## Function `generate_linear_transfer_ref`

Create a LinearTransferRef for a one-time transfer. This requires that the owner at the
time of generation is the owner at the time of transferring.


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;object::TransferRef): object::LinearTransferRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;TransferRef): LinearTransferRef acquires ObjectCore &#123;
    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));
    let owner &#61; owner(Object&lt;ObjectCore&gt; &#123; inner: ref.self &#125;);
    LinearTransferRef &#123;
        self: ref.self,
        owner,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer to the destination address using a LinearTransferRef.


<pre><code>public fun transfer_with_ref(ref: object::LinearTransferRef, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_with_ref(ref: LinearTransferRef, to: address) acquires ObjectCore &#123;
    assert!(!exists&lt;Untransferable&gt;(ref.self), error::permission_denied(ENOT_MOVABLE));
    let object &#61; borrow_global_mut&lt;ObjectCore&gt;(ref.self);
    assert!(
        object.owner &#61;&#61; ref.owner,
        error::permission_denied(ENOT_OBJECT_OWNER),
    );
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            Transfer &#123;
                object: ref.self,
                from: object.owner,
                to,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut object.transfer_events,
        TransferEvent &#123;
            object: ref.self,
            from: object.owner,
            to,
        &#125;,
    );
    object.owner &#61; to;
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer_call"></a>

## Function `transfer_call`

Entry function that can be used to transfer, if allow_ungated_transfer is set true.


<pre><code>public entry fun transfer_call(owner: &amp;signer, object: address, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_call(
    owner: &amp;signer,
    object: address,
    to: address,
) acquires ObjectCore &#123;
    transfer_raw(owner, object, to)
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer"></a>

## Function `transfer`

Transfers ownership of the object (and all associated resources) at the specified address
for Object<T> to the "to" address.


<pre><code>public entry fun transfer&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(
    owner: &amp;signer,
    object: Object&lt;T&gt;,
    to: address,
) acquires ObjectCore &#123;
    transfer_raw(owner, object.inner, to)
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer_raw"></a>

## Function `transfer_raw`

Attempts to transfer using addresses only. Transfers the given object if
allow_ungated_transfer is set true. Note, that this allows the owner of a nested object to
transfer that object, so long as allow_ungated_transfer is enabled at each stage in the
hierarchy.


<pre><code>public fun transfer_raw(owner: &amp;signer, object: address, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_raw(
    owner: &amp;signer,
    object: address,
    to: address,
) acquires ObjectCore &#123;
    let owner_address &#61; signer::address_of(owner);
    verify_ungated_and_descendant(owner_address, object);
    transfer_raw_inner(object, to);
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer_raw_inner"></a>

## Function `transfer_raw_inner`



<pre><code>fun transfer_raw_inner(object: address, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun transfer_raw_inner(object: address, to: address) acquires ObjectCore &#123;
    let object_core &#61; borrow_global_mut&lt;ObjectCore&gt;(object);
    if (object_core.owner !&#61; to) &#123;
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Transfer &#123;
                    object,
                    from: object_core.owner,
                    to,
                &#125;,
            );
        &#125;;
        event::emit_event(
            &amp;mut object_core.transfer_events,
            TransferEvent &#123;
                object,
                from: object_core.owner,
                to,
            &#125;,
        );
        object_core.owner &#61; to;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_object_transfer_to_object"></a>

## Function `transfer_to_object`

Transfer the given object to another object. See <code>transfer</code> for more information.


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(owner: &amp;signer, object: object::Object&lt;O&gt;, to: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(
    owner: &amp;signer,
    object: Object&lt;O&gt;,
    to: Object&lt;T&gt;,
) acquires ObjectCore &#123;
    transfer(owner, object, to.inner)
&#125;
</code></pre>



</details>

<a id="0x1_object_verify_ungated_and_descendant"></a>

## Function `verify_ungated_and_descendant`

This checks that the destination address is eventually owned by the owner and that each
object between the two allows for ungated transfers. Note, this is limited to a depth of 8
objects may have cyclic dependencies.


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address) acquires ObjectCore &#123;
    let current_address &#61; destination;
    assert!(
        exists&lt;ObjectCore&gt;(current_address),
        error::not_found(EOBJECT_DOES_NOT_EXIST),
    );

    let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);
    assert!(
        object.allow_ungated_transfer,
        error::permission_denied(ENO_UNGATED_TRANSFERS),
    );

    let current_address &#61; object.owner;
    let count &#61; 0;
    while (owner !&#61; current_address) &#123;
        count &#61; count &#43; 1;
        if (std::features::max_object_nesting_check_enabled()) &#123;
            assert!(count &lt; MAXIMUM_OBJECT_NESTING, error::out_of_range(EMAXIMUM_NESTING))
        &#125;;
        // At this point, the first object exists and so the more likely case is that the
        // object&apos;s owner is not an object. So we return a more sensible error.
        assert!(
            exists&lt;ObjectCore&gt;(current_address),
            error::permission_denied(ENOT_OBJECT_OWNER),
        );
        let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);
        assert!(
            object.allow_ungated_transfer,
            error::permission_denied(ENO_UNGATED_TRANSFERS),
        );
        current_address &#61; object.owner;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_object_burn"></a>

## Function `burn`

Forcefully transfer an unwanted object to BURN_ADDRESS, ignoring whether ungated_transfer is allowed.
This only works for objects directly owned and for simplicity does not apply to indirectly owned objects.
Original owners can reclaim burnt objects any time in the future by calling unburn.


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: Object&lt;T&gt;) acquires ObjectCore &#123;
    let original_owner &#61; signer::address_of(owner);
    assert!(is_owner(object, original_owner), error::permission_denied(ENOT_OBJECT_OWNER));
    let object_addr &#61; object.inner;
    move_to(&amp;create_signer(object_addr), TombStone &#123; original_owner &#125;);
    transfer_raw_inner(object_addr, BURN_ADDRESS);
&#125;
</code></pre>



</details>

<a id="0x1_object_unburn"></a>

## Function `unburn`

Allow origin owners to reclaim any objects they previous burnt.


<pre><code>public entry fun unburn&lt;T: key&gt;(original_owner: &amp;signer, object: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unburn&lt;T: key&gt;(
    original_owner: &amp;signer,
    object: Object&lt;T&gt;,
) acquires TombStone, ObjectCore &#123;
    let object_addr &#61; object.inner;
    assert!(exists&lt;TombStone&gt;(object_addr), error::invalid_argument(EOBJECT_NOT_BURNT));

    let TombStone &#123; original_owner: original_owner_addr &#125; &#61; move_from&lt;TombStone&gt;(object_addr);
    assert!(original_owner_addr &#61;&#61; signer::address_of(original_owner), error::permission_denied(ENOT_OBJECT_OWNER));
    transfer_raw_inner(object_addr, original_owner_addr);
&#125;
</code></pre>



</details>

<a id="0x1_object_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Accessors
Return true if ungated transfer is allowed.


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: Object&lt;T&gt;): bool acquires ObjectCore &#123;
    assert!(
        exists&lt;ObjectCore&gt;(object.inner),
        error::not_found(EOBJECT_DOES_NOT_EXIST),
    );
    borrow_global&lt;ObjectCore&gt;(object.inner).allow_ungated_transfer
&#125;
</code></pre>



</details>

<a id="0x1_object_owner"></a>

## Function `owner`

Return the current owner.


<pre><code>public fun owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owner&lt;T: key&gt;(object: Object&lt;T&gt;): address acquires ObjectCore &#123;
    assert!(
        exists&lt;ObjectCore&gt;(object.inner),
        error::not_found(EOBJECT_DOES_NOT_EXIST),
    );
    borrow_global&lt;ObjectCore&gt;(object.inner).owner
&#125;
</code></pre>



</details>

<a id="0x1_object_is_owner"></a>

## Function `is_owner`

Return true if the provided address is the current owner.


<pre><code>public fun is_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_owner&lt;T: key&gt;(object: Object&lt;T&gt;, owner: address): bool acquires ObjectCore &#123;
    owner(object) &#61;&#61; owner
&#125;
</code></pre>



</details>

<a id="0x1_object_owns"></a>

## Function `owns`

Return true if the provided address has indirect or direct ownership of the provided object.


<pre><code>public fun owns&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owns&lt;T: key&gt;(object: Object&lt;T&gt;, owner: address): bool acquires ObjectCore &#123;
    let current_address &#61; object_address(&amp;object);
    if (current_address &#61;&#61; owner) &#123;
        return true
    &#125;;

    assert!(
        exists&lt;ObjectCore&gt;(current_address),
        error::not_found(EOBJECT_DOES_NOT_EXIST),
    );

    let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);
    let current_address &#61; object.owner;

    let count &#61; 0;
    while (owner !&#61; current_address) &#123;
        count &#61; count &#43; 1;
        if (std::features::max_object_nesting_check_enabled()) &#123;
            assert!(count &lt; MAXIMUM_OBJECT_NESTING, error::out_of_range(EMAXIMUM_NESTING))
        &#125;;
        if (!exists&lt;ObjectCore&gt;(current_address)) &#123;
            return false
        &#125;;

        let object &#61; borrow_global&lt;ObjectCore&gt;(current_address);
        current_address &#61; object.owner;
    &#125;;
    true
&#125;
</code></pre>



</details>

<a id="0x1_object_root_owner"></a>

## Function `root_owner`

Returns the root owner of an object. As objects support nested ownership, it can be useful
to determine the identity of the starting point of ownership.


<pre><code>public fun root_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun root_owner&lt;T: key&gt;(object: Object&lt;T&gt;): address acquires ObjectCore &#123;
    let obj_owner &#61; owner(object);
    while (is_object(obj_owner)) &#123;
        obj_owner &#61; owner(address_to_object&lt;ObjectCore&gt;(obj_owner));
    &#125;;
    obj_owner
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>It's not possible to create an object twice on the same address.</td>
<td>Critical</td>
<td>The create_object_internal function includes an assertion to ensure that the object being created does not already exist at the specified address.</td>
<td>Formally verified via <a href="#high-level-req-1">create_object_internal</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Only its owner may transfer an object.</td>
<td>Critical</td>
<td>The transfer function mandates that the transaction be signed by the owner's address, ensuring that only the rightful owner may initiate the object transfer.</td>
<td>Audited that it aborts if anyone other than the owner attempts to transfer.</td>
</tr>

<tr>
<td>3</td>
<td>The indirect owner of an object may transfer the object.</td>
<td>Medium</td>
<td>The owns function evaluates to true when the given address possesses either direct or indirect ownership of the specified object.</td>
<td>Audited that it aborts if address transferring is not indirect owner.</td>
</tr>

<tr>
<td>4</td>
<td>Objects may never change the address which houses them.</td>
<td>Low</td>
<td>After creating an object, transfers to another owner may occur. However, the address which stores the object may not be changed.</td>
<td>This is implied by <a href="#high-level-req">high-level requirement 1</a>.</td>
</tr>

<tr>
<td>5</td>
<td>If an ungated transfer is disabled on an object in an indirect ownership chain, a transfer should not occur.</td>
<td>Medium</td>
<td>Calling disable_ungated_transfer disables direct transfer, and only TransferRef may trigger transfers. The transfer_with_ref function is called.</td>
<td>Formally verified via <a href="#high-level-req-5">transfer_with_ref</a>.</td>
</tr>

<tr>
<td>6</td>
<td>Object addresses must not overlap with other addresses in different domains.</td>
<td>Critical</td>
<td>The current addressing scheme with suffixes does not conflict with any existing addresses, such as resource accounts. The GUID space is explicitly separated to ensure this doesn't happen.</td>
<td>This is true by construction if one correctly ensures the usage of INIT_GUID_CREATION_NUM during the creation of GUID.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma aborts_if_is_strict;
</code></pre>




<a id="0x1_object_spec_exists_at"></a>


<pre><code>fun spec_exists_at&lt;T: key&gt;(object: address): bool;
</code></pre>



<a id="@Specification_1_address_to_object"></a>

### Function `address_to_object`


<pre><code>public fun address_to_object&lt;T: key&gt;(object: address): object::Object&lt;T&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object);
aborts_if !spec_exists_at&lt;T&gt;(object);
ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: object &#125;;
</code></pre>



<a id="@Specification_1_create_object_address"></a>

### Function `create_object_address`


<pre><code>public fun create_object_address(source: &amp;address, seed: vector&lt;u8&gt;): address
</code></pre>




<pre><code>pragma opaque;
pragma aborts_if_is_strict &#61; false;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_create_object_address(source, seed);
</code></pre>




<a id="0x1_object_spec_create_user_derived_object_address_impl"></a>


<pre><code>fun spec_create_user_derived_object_address_impl(source: address, derive_from: address): address;
</code></pre>



<a id="@Specification_1_create_user_derived_object_address_impl"></a>

### Function `create_user_derived_object_address_impl`


<pre><code>fun create_user_derived_object_address_impl(source: address, derive_from: address): address
</code></pre>




<pre><code>pragma opaque;
ensures [abstract] result &#61;&#61; spec_create_user_derived_object_address_impl(source, derive_from);
</code></pre>



<a id="@Specification_1_create_user_derived_object_address"></a>

### Function `create_user_derived_object_address`


<pre><code>public fun create_user_derived_object_address(source: address, derive_from: address): address
</code></pre>




<pre><code>pragma opaque;
pragma aborts_if_is_strict &#61; false;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_create_user_derived_object_address(source, derive_from);
</code></pre>



<a id="@Specification_1_create_guid_object_address"></a>

### Function `create_guid_object_address`


<pre><code>public fun create_guid_object_address(source: address, creation_num: u64): address
</code></pre>




<pre><code>pragma opaque;
pragma aborts_if_is_strict &#61; false;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_create_guid_object_address(source, creation_num);
</code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code>fun exists_at&lt;T: key&gt;(object: address): bool
</code></pre>




<pre><code>pragma opaque;
ensures [abstract] result &#61;&#61; spec_exists_at&lt;T&gt;(object);
</code></pre>



<a id="@Specification_1_object_address"></a>

### Function `object_address`


<pre><code>public fun object_address&lt;T: key&gt;(object: &amp;object::Object&lt;T&gt;): address
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; object.inner;
</code></pre>



<a id="@Specification_1_convert"></a>

### Function `convert`


<pre><code>public fun convert&lt;X: key, Y: key&gt;(object: object::Object&lt;X&gt;): object::Object&lt;Y&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);
aborts_if !spec_exists_at&lt;Y&gt;(object.inner);
ensures result &#61;&#61; Object&lt;Y&gt; &#123; inner: object.inner &#125;;
</code></pre>



<a id="@Specification_1_create_named_object"></a>

### Function `create_named_object`


<pre><code>public fun create_named_object(creator: &amp;signer, seed: vector&lt;u8&gt;): object::ConstructorRef
</code></pre>




<pre><code>let creator_address &#61; signer::address_of(creator);
let obj_addr &#61; spec_create_object_address(creator_address, seed);
aborts_if exists&lt;ObjectCore&gt;(obj_addr);
ensures exists&lt;ObjectCore&gt;(obj_addr);
ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: creator_address,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: obj_addr,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: false &#125;;
</code></pre>



<a id="@Specification_1_create_user_derived_object"></a>

### Function `create_user_derived_object`


<pre><code>public(friend) fun create_user_derived_object(creator_address: address, derive_ref: &amp;object::DeriveRef): object::ConstructorRef
</code></pre>




<pre><code>let obj_addr &#61; spec_create_user_derived_object_address(creator_address, derive_ref.self);
aborts_if exists&lt;ObjectCore&gt;(obj_addr);
ensures exists&lt;ObjectCore&gt;(obj_addr);
ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: creator_address,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: obj_addr,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: false &#125;;
</code></pre>



<a id="@Specification_1_create_object"></a>

### Function `create_object`


<pre><code>public fun create_object(owner_address: address): object::ConstructorRef
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let unique_address &#61; transaction_context::spec_generate_unique_address();
aborts_if exists&lt;ObjectCore&gt;(unique_address);
ensures exists&lt;ObjectCore&gt;(unique_address);
ensures global&lt;ObjectCore&gt;(unique_address) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: owner_address,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: unique_address,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: unique_address, can_delete: true &#125;;
</code></pre>



<a id="@Specification_1_create_sticky_object"></a>

### Function `create_sticky_object`


<pre><code>public fun create_sticky_object(owner_address: address): object::ConstructorRef
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let unique_address &#61; transaction_context::spec_generate_unique_address();
aborts_if exists&lt;ObjectCore&gt;(unique_address);
ensures exists&lt;ObjectCore&gt;(unique_address);
ensures global&lt;ObjectCore&gt;(unique_address) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: owner_address,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: unique_address,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: unique_address, can_delete: false &#125;;
</code></pre>



<a id="@Specification_1_create_sticky_object_at_address"></a>

### Function `create_sticky_object_at_address`


<pre><code>public(friend) fun create_sticky_object_at_address(owner_address: address, object_address: address): object::ConstructorRef
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_create_object_from_account"></a>

### Function `create_object_from_account`


<pre><code>&#35;[deprecated]
public fun create_object_from_account(creator: &amp;signer): object::ConstructorRef
</code></pre>




<pre><code>aborts_if !exists&lt;account::Account&gt;(signer::address_of(creator));
let object_data &#61; global&lt;account::Account&gt;(signer::address_of(creator));
aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;
aborts_if object_data.guid_creation_num &#43; 1 &gt;&#61; account::MAX_GUID_CREATION_NUM;
let creation_num &#61; object_data.guid_creation_num;
let addr &#61; signer::address_of(creator);
let guid &#61; guid::GUID &#123;
    id: guid::ID &#123;
        creation_num,
        addr,
    &#125;
&#125;;
let bytes_spec &#61; bcs::to_bytes(guid);
let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));
let hash_bytes &#61; hash::sha3_256(bytes);
let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);
aborts_if exists&lt;ObjectCore&gt;(obj_addr);
aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);
ensures global&lt;account::Account&gt;(addr).guid_creation_num &#61;&#61; old(
    global&lt;account::Account&gt;(addr)
).guid_creation_num &#43; 1;
ensures exists&lt;ObjectCore&gt;(obj_addr);
ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: addr,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: obj_addr,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;
</code></pre>



<a id="@Specification_1_create_object_from_object"></a>

### Function `create_object_from_object`


<pre><code>&#35;[deprecated]
public fun create_object_from_object(creator: &amp;signer): object::ConstructorRef
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(creator));
let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(creator));
aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;
let creation_num &#61; object_data.guid_creation_num;
let addr &#61; signer::address_of(creator);
let guid &#61; guid::GUID &#123;
    id: guid::ID &#123;
        creation_num,
        addr,
    &#125;
&#125;;
let bytes_spec &#61; bcs::to_bytes(guid);
let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));
let hash_bytes &#61; hash::sha3_256(bytes);
let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);
aborts_if exists&lt;ObjectCore&gt;(obj_addr);
aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);
ensures global&lt;ObjectCore&gt;(addr).guid_creation_num &#61;&#61; old(global&lt;ObjectCore&gt;(addr)).guid_creation_num &#43; 1;
ensures exists&lt;ObjectCore&gt;(obj_addr);
ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: addr,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: obj_addr,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;
</code></pre>



<a id="@Specification_1_create_object_from_guid"></a>

### Function `create_object_from_guid`


<pre><code>fun create_object_from_guid(creator_address: address, guid: guid::GUID): object::ConstructorRef
</code></pre>




<pre><code>let bytes_spec &#61; bcs::to_bytes(guid);
let bytes &#61; concat(bytes_spec, vec&lt;u8&gt;(OBJECT_FROM_GUID_ADDRESS_SCHEME));
let hash_bytes &#61; hash::sha3_256(bytes);
let obj_addr &#61; from_bcs::deserialize&lt;address&gt;(hash_bytes);
aborts_if exists&lt;ObjectCore&gt;(obj_addr);
aborts_if !from_bcs::deserializable&lt;address&gt;(hash_bytes);
ensures exists&lt;ObjectCore&gt;(obj_addr);
ensures global&lt;ObjectCore&gt;(obj_addr) &#61;&#61; ObjectCore &#123;
    guid_creation_num: INIT_GUID_CREATION_NUM &#43; 1,
    owner: creator_address,
    allow_ungated_transfer: true,
    transfer_events: event::EventHandle &#123;
        counter: 0,
        guid: guid::GUID &#123;
            id: guid::ID &#123;
                creation_num: INIT_GUID_CREATION_NUM,
                addr: obj_addr,
            &#125;
        &#125;
    &#125;
&#125;;
ensures result &#61;&#61; ConstructorRef &#123; self: obj_addr, can_delete: true &#125;;
</code></pre>



<a id="@Specification_1_create_object_internal"></a>

### Function `create_object_internal`


<pre><code>fun create_object_internal(creator_address: address, object: address, can_delete: bool): object::ConstructorRef
</code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
aborts_if exists&lt;ObjectCore&gt;(object);
ensures exists&lt;ObjectCore&gt;(object);
ensures global&lt;ObjectCore&gt;(object).guid_creation_num &#61;&#61; INIT_GUID_CREATION_NUM &#43; 1;
ensures result &#61;&#61; ConstructorRef &#123; self: object, can_delete &#125;;
</code></pre>



<a id="@Specification_1_generate_delete_ref"></a>

### Function `generate_delete_ref`


<pre><code>public fun generate_delete_ref(ref: &amp;object::ConstructorRef): object::DeleteRef
</code></pre>




<pre><code>aborts_if !ref.can_delete;
ensures result &#61;&#61; DeleteRef &#123; self: ref.self &#125;;
</code></pre>



<a id="@Specification_1_generate_transfer_ref"></a>

### Function `generate_transfer_ref`


<pre><code>public fun generate_transfer_ref(ref: &amp;object::ConstructorRef): object::TransferRef
</code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);
ensures result &#61;&#61; TransferRef &#123;
    self: ref.self,
&#125;;
</code></pre>



<a id="@Specification_1_object_from_constructor_ref"></a>

### Function `object_from_constructor_ref`


<pre><code>public fun object_from_constructor_ref&lt;T: key&gt;(ref: &amp;object::ConstructorRef): object::Object&lt;T&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);
aborts_if !spec_exists_at&lt;T&gt;(ref.self);
ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: ref.self &#125;;
</code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code>public fun create_guid(object: &amp;signer): guid::GUID
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(object));
let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(object));
aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;
ensures result &#61;&#61; guid::GUID &#123;
    id: guid::ID &#123;
        creation_num: object_data.guid_creation_num,
        addr: signer::address_of(object)
    &#125;
&#125;;
</code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(object: &amp;signer): event::EventHandle&lt;T&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(signer::address_of(object));
let object_data &#61; global&lt;ObjectCore&gt;(signer::address_of(object));
aborts_if object_data.guid_creation_num &#43; 1 &gt; MAX_U64;
let guid &#61; guid::GUID &#123;
    id: guid::ID &#123;
        creation_num: object_data.guid_creation_num,
        addr: signer::address_of(object)
    &#125;
&#125;;
ensures result &#61;&#61; event::EventHandle&lt;T&gt; &#123;
    counter: 0,
    guid,
&#125;;
</code></pre>



<a id="@Specification_1_object_from_delete_ref"></a>

### Function `object_from_delete_ref`


<pre><code>public fun object_from_delete_ref&lt;T: key&gt;(ref: &amp;object::DeleteRef): object::Object&lt;T&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);
aborts_if !spec_exists_at&lt;T&gt;(ref.self);
ensures result &#61;&#61; Object&lt;T&gt; &#123; inner: ref.self &#125;;
</code></pre>



<a id="@Specification_1_delete"></a>

### Function `delete`


<pre><code>public fun delete(ref: object::DeleteRef)
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);
ensures !exists&lt;ObjectCore&gt;(ref.self);
</code></pre>



<a id="@Specification_1_disable_ungated_transfer"></a>

### Function `disable_ungated_transfer`


<pre><code>public fun disable_ungated_transfer(ref: &amp;object::TransferRef)
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);
ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; false;
</code></pre>



<a id="@Specification_1_set_untransferable"></a>

### Function `set_untransferable`


<pre><code>public fun set_untransferable(ref: &amp;object::ConstructorRef)
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(ref.self);
aborts_if exists&lt;Untransferable&gt;(ref.self);
ensures exists&lt;Untransferable&gt;(ref.self);
ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; false;
</code></pre>



<a id="@Specification_1_enable_ungated_transfer"></a>

### Function `enable_ungated_transfer`


<pre><code>public fun enable_ungated_transfer(ref: &amp;object::TransferRef)
</code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);
aborts_if !exists&lt;ObjectCore&gt;(ref.self);
ensures global&lt;ObjectCore&gt;(ref.self).allow_ungated_transfer &#61;&#61; true;
</code></pre>



<a id="@Specification_1_generate_linear_transfer_ref"></a>

### Function `generate_linear_transfer_ref`


<pre><code>public fun generate_linear_transfer_ref(ref: &amp;object::TransferRef): object::LinearTransferRef
</code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);
aborts_if !exists&lt;ObjectCore&gt;(ref.self);
let owner &#61; global&lt;ObjectCore&gt;(ref.self).owner;
ensures result &#61;&#61; LinearTransferRef &#123;
    self: ref.self,
    owner,
&#125;;
</code></pre>



<a id="@Specification_1_transfer_with_ref"></a>

### Function `transfer_with_ref`


<pre><code>public fun transfer_with_ref(ref: object::LinearTransferRef, to: address)
</code></pre>




<pre><code>aborts_if exists&lt;Untransferable&gt;(ref.self);
let object &#61; global&lt;ObjectCore&gt;(ref.self);
aborts_if !exists&lt;ObjectCore&gt;(ref.self);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
aborts_if object.owner !&#61; ref.owner;
ensures global&lt;ObjectCore&gt;(ref.self).owner &#61;&#61; to;
</code></pre>



<a id="@Specification_1_transfer_call"></a>

### Function `transfer_call`


<pre><code>public entry fun transfer_call(owner: &amp;signer, object: address, to: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let owner_address &#61; signer::address_of(owner);
aborts_if !exists&lt;ObjectCore&gt;(object);
aborts_if !global&lt;ObjectCore&gt;(object).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;, to: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let owner_address &#61; signer::address_of(owner);
let object_address &#61; object.inner;
aborts_if !exists&lt;ObjectCore&gt;(object_address);
aborts_if !global&lt;ObjectCore&gt;(object_address).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer_raw"></a>

### Function `transfer_raw`


<pre><code>public fun transfer_raw(owner: &amp;signer, object: address, to: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let owner_address &#61; signer::address_of(owner);
aborts_if !exists&lt;ObjectCore&gt;(object);
aborts_if !global&lt;ObjectCore&gt;(object).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer_to_object"></a>

### Function `transfer_to_object`


<pre><code>public entry fun transfer_to_object&lt;O: key, T: key&gt;(owner: &amp;signer, object: object::Object&lt;O&gt;, to: object::Object&lt;T&gt;)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let owner_address &#61; signer::address_of(owner);
let object_address &#61; object.inner;
aborts_if !exists&lt;ObjectCore&gt;(object_address);
aborts_if !global&lt;ObjectCore&gt;(object_address).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_verify_ungated_and_descendant"></a>

### Function `verify_ungated_and_descendant`


<pre><code>fun verify_ungated_and_descendant(owner: address, destination: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
pragma unroll &#61; MAXIMUM_OBJECT_NESTING;
aborts_if !exists&lt;ObjectCore&gt;(destination);
aborts_if !global&lt;ObjectCore&gt;(destination).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public entry fun burn&lt;T: key&gt;(owner: &amp;signer, object: object::Object&lt;T&gt;)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let object_address &#61; object.inner;
aborts_if !exists&lt;ObjectCore&gt;(object_address);
aborts_if owner(object) !&#61; signer::address_of(owner);
aborts_if is_burnt(object);
</code></pre>



<a id="@Specification_1_unburn"></a>

### Function `unburn`


<pre><code>public entry fun unburn&lt;T: key&gt;(original_owner: &amp;signer, object: object::Object&lt;T&gt;)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let object_address &#61; object.inner;
aborts_if !exists&lt;ObjectCore&gt;(object_address);
aborts_if !is_burnt(object);
let tomb_stone &#61; borrow_global&lt;TombStone&gt;(object_address);
aborts_if tomb_stone.original_owner !&#61; signer::address_of(original_owner);
</code></pre>



<a id="@Specification_1_ungated_transfer_allowed"></a>

### Function `ungated_transfer_allowed`


<pre><code>public fun ungated_transfer_allowed&lt;T: key&gt;(object: object::Object&lt;T&gt;): bool
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);
ensures result &#61;&#61; global&lt;ObjectCore&gt;(object.inner).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_owner"></a>

### Function `owner`


<pre><code>public fun owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);
ensures result &#61;&#61; global&lt;ObjectCore&gt;(object.inner).owner;
</code></pre>



<a id="@Specification_1_is_owner"></a>

### Function `is_owner`


<pre><code>public fun is_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool
</code></pre>




<pre><code>aborts_if !exists&lt;ObjectCore&gt;(object.inner);
ensures result &#61;&#61; (global&lt;ObjectCore&gt;(object.inner).owner &#61;&#61; owner);
</code></pre>



<a id="@Specification_1_owns"></a>

### Function `owns`


<pre><code>public fun owns&lt;T: key&gt;(object: object::Object&lt;T&gt;, owner: address): bool
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let current_address_0 &#61; object.inner;
let object_0 &#61; global&lt;ObjectCore&gt;(current_address_0);
let current_address &#61; object_0.owner;
aborts_if object.inner !&#61; owner &amp;&amp; !exists&lt;ObjectCore&gt;(object.inner);
ensures current_address_0 &#61;&#61; owner &#61;&#61;&gt; result &#61;&#61; true;
</code></pre>



<a id="@Specification_1_root_owner"></a>

### Function `root_owner`


<pre><code>public fun root_owner&lt;T: key&gt;(object: object::Object&lt;T&gt;): address
</code></pre>




<pre><code>pragma aborts_if_is_partial;
</code></pre>




<a id="0x1_object_spec_create_object_address"></a>


<pre><code>fun spec_create_object_address(source: address, seed: vector&lt;u8&gt;): address;
</code></pre>




<a id="0x1_object_spec_create_user_derived_object_address"></a>


<pre><code>fun spec_create_user_derived_object_address(source: address, derive_from: address): address;
</code></pre>




<a id="0x1_object_spec_create_guid_object_address"></a>


<pre><code>fun spec_create_guid_object_address(source: address, creation_num: u64): address;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
