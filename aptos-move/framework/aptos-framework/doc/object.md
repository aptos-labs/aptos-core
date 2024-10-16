
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_object_ObjectCore"></a>

## Resource `ObjectCore`

The core of the object model that defines ownership, transferability, and events.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> <b>has</b> key
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
 transfers bypassing the use of a TransferRef.
</dd>
<dt>
<code>transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="object.md#0x1_object_TransferEvent">object::TransferEvent</a>&gt;</code>
</dt>
<dd>
 Emitted events upon transferring of ownership.
</dd>
</dl>


</details>

<a id="0x1_object_TombStone"></a>

## Resource `TombStone`

This is added to objects that are burnt (ownership transferred to BURN_ADDRESS).


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object.md#0x1_object_TombStone">TombStone</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>original_owner: <b>address</b></code>
</dt>
<dd>
 Track the previous owner before the object is burnt so they can reclaim later if so desired.
</dd>
</dl>


</details>

<a id="0x1_object_Untransferable"></a>

## Resource `Untransferable`

The existence of this renders all <code><a href="object.md#0x1_object_TransferRef">TransferRef</a></code>s irrelevant. The object cannot be moved.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object.md#0x1_object_Untransferable">Untransferable</a> <b>has</b> key
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


<pre><code>#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="object.md#0x1_object_ObjectGroup">ObjectGroup</a>
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


<pre><code><b>struct</b> <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_object_ConstructorRef"></a>

## Struct `ConstructorRef`

This is a one time ability given to the creator to configure the object as necessary


<pre><code><b>struct</b> <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> <b>has</b> drop, store
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

<a id="0x1_object_ExtendRef"></a>

## Struct `ExtendRef`

Used to create events or move additional resources into object storage.


<pre><code><b>struct</b> <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> <b>has</b> drop, store
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

<a id="0x1_object_TransferRef"></a>

## Struct `TransferRef`

Used to create LinearTransferRef, hence ownership transfer.


<pre><code><b>struct</b> <a href="object.md#0x1_object_TransferRef">TransferRef</a> <b>has</b> drop, store
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

<a id="0x1_object_LinearTransferRef"></a>

## Struct `LinearTransferRef`

Used to perform transfers. This locks transferring ability to a single time use bound to
the current owner.


<pre><code><b>struct</b> <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <b>address</b></code>
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

<a id="0x1_object_DeriveRef"></a>

## Struct `DeriveRef`

Used to create derived objects from a given objects.


<pre><code><b>struct</b> <a href="object.md#0x1_object_DeriveRef">DeriveRef</a> <b>has</b> drop, store
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

<a id="0x1_object_TransferEvent"></a>

## Struct `TransferEvent`

Emitted whenever the object's owner field is changed.


<pre><code><b>struct</b> <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="object.md#0x1_object">object</a>: <b>address</b></code>
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

<a id="0x1_object_Transfer"></a>

## Struct `Transfer`

Emitted whenever the object's owner field is changed.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="object.md#0x1_object_Transfer">Transfer</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="object.md#0x1_object">object</a>: <b>address</b></code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_object_BURN_ADDRESS"></a>

Address where unwanted objects can be forcefully transferred to.


<pre><code><b>const</b> <a href="object.md#0x1_object_BURN_ADDRESS">BURN_ADDRESS</a>: <b>address</b> = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
</code></pre>



<a id="0x1_object_DERIVE_AUID_ADDRESS_SCHEME"></a>

generate_unique_address uses this for domain separation within its native implementation


<pre><code><b>const</b> <a href="object.md#0x1_object_DERIVE_AUID_ADDRESS_SCHEME">DERIVE_AUID_ADDRESS_SCHEME</a>: u8 = 251;
</code></pre>



<a id="0x1_object_EBURN_NOT_ALLOWED"></a>

Objects cannot be burnt


<pre><code><b>const</b> <a href="object.md#0x1_object_EBURN_NOT_ALLOWED">EBURN_NOT_ALLOWED</a>: u64 = 10;
</code></pre>



<a id="0x1_object_ECANNOT_DELETE"></a>

The object does not allow for deletion


<pre><code><b>const</b> <a href="object.md#0x1_object_ECANNOT_DELETE">ECANNOT_DELETE</a>: u64 = 5;
</code></pre>



<a id="0x1_object_EMAXIMUM_NESTING"></a>

Exceeds maximum nesting for an object transfer.


<pre><code><b>const</b> <a href="object.md#0x1_object_EMAXIMUM_NESTING">EMAXIMUM_NESTING</a>: u64 = 6;
</code></pre>



<a id="0x1_object_ENOT_OBJECT_OWNER"></a>

The caller does not have ownership permissions


<pre><code><b>const</b> <a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>: u64 = 4;
</code></pre>



<a id="0x1_object_ENO_UNGATED_TRANSFERS"></a>

The object does not have ungated transfers enabled


<pre><code><b>const</b> <a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>: u64 = 3;
</code></pre>



<a id="0x1_object_EOBJECT_DOES_NOT_EXIST"></a>

An object does not exist at this address


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a id="0x1_object_EOBJECT_EXISTS"></a>

An object already exists at this address


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_EXISTS">EOBJECT_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x1_object_EOBJECT_NOT_BURNT"></a>

Cannot reclaim objects that weren't burnt.


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_NOT_BURNT">EOBJECT_NOT_BURNT</a>: u64 = 8;
</code></pre>



<a id="0x1_object_EOBJECT_NOT_TRANSFERRABLE"></a>

Object is untransferable any operations that might result in a transfer are disallowed.


<pre><code><b>const</b> <a href="object.md#0x1_object_EOBJECT_NOT_TRANSFERRABLE">EOBJECT_NOT_TRANSFERRABLE</a>: u64 = 9;
</code></pre>



<a id="0x1_object_ERESOURCE_DOES_NOT_EXIST"></a>

The resource is not stored at the specified address.


<pre><code><b>const</b> <a href="object.md#0x1_object_ERESOURCE_DOES_NOT_EXIST">ERESOURCE_DOES_NOT_EXIST</a>: u64 = 7;
</code></pre>



<a id="0x1_object_INIT_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code><b>const</b> <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>: u64 = 1125899906842624;
</code></pre>



<a id="0x1_object_MAXIMUM_OBJECT_NESTING"></a>

Maximum nesting from one object to another. That is objects can technically have infinte
nesting, but any checks such as transfer will only be evaluated this deep.


<pre><code><b>const</b> <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>: u8 = 8;
</code></pre>



<a id="0x1_object_OBJECT_DERIVED_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> as derived from another object.
The object's address is generated as:
```
obj_addr = sha3_256(account addr | derived from object's address | 0xFC)
```

This 0xFC constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code><b>const</b> <a href="object.md#0x1_object_OBJECT_DERIVED_SCHEME">OBJECT_DERIVED_SCHEME</a>: u8 = 252;
</code></pre>



<a id="0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> via a fresh GUID generated by the creator at
<code>source_addr</code>. The object's address is generated as:
```
obj_addr = sha3_256(guid | 0xFD)
```
where <code><a href="guid.md#0x1_guid">guid</a> = <a href="account.md#0x1_account_create_guid">account::create_guid</a>(<a href="create_signer.md#0x1_create_signer">create_signer</a>(source_addr))</code>

This 0xFD constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code><b>const</b> <a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>: u8 = 253;
</code></pre>



<a id="0x1_object_OBJECT_FROM_SEED_ADDRESS_SCHEME"></a>

Scheme identifier used to generate an object's address <code>obj_addr</code> from the creator's <code>source_addr</code> and a <code>seed</code> as:
obj_addr = sha3_256(source_addr | seed | 0xFE).

This 0xFE constant serves as a domain separation tag to prevent existing authentication key and resource account
derivation to produce an object address.


<pre><code><b>const</b> <a href="object.md#0x1_object_OBJECT_FROM_SEED_ADDRESS_SCHEME">OBJECT_FROM_SEED_ADDRESS_SCHEME</a>: u8 = 254;
</code></pre>



<a id="0x1_object_is_untransferable"></a>

## Function `is_untransferable`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_untransferable">is_untransferable</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_untransferable">is_untransferable</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): bool {
    <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(<a href="object.md#0x1_object">object</a>.inner)
}
</code></pre>



</details>

<a id="0x1_object_is_burnt"></a>

## Function `is_burnt`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_burnt">is_burnt</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_burnt">is_burnt</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): bool {
    <b>exists</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(<a href="object.md#0x1_object">object</a>.inner)
}
</code></pre>



</details>

<a id="0x1_object_address_to_object"></a>

## Function `address_to_object`

Produces an ObjectId from the given address. This is not verified.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; {
    <b>assert</b>!(<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>));
    <b>assert</b>!(<a href="object.md#0x1_object_exists_at">exists_at</a>&lt;T&gt;(<a href="object.md#0x1_object">object</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_ERESOURCE_DOES_NOT_EXIST">ERESOURCE_DOES_NOT_EXIST</a>));
    <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; { inner: <a href="object.md#0x1_object">object</a> }
}
</code></pre>



</details>

<a id="0x1_object_is_object"></a>

## Function `is_object`

Returns true if there exists an object or the remnants of an object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_object">is_object</a>(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_object">is_object</a>(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>)
}
</code></pre>



</details>

<a id="0x1_object_object_exists"></a>

## Function `object_exists`

Returns true if there exists an object with resource T.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_exists">object_exists</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_exists">object_exists</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>) && <a href="object.md#0x1_object_exists_at">exists_at</a>&lt;T&gt;(<a href="object.md#0x1_object">object</a>)
}
</code></pre>



</details>

<a id="0x1_object_create_object_address"></a>

## Function `create_object_address`

Derives an object address from source material: sha3_256([creator address | seed | 0xFE]).


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_address">create_object_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_address">create_object_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(source);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, seed);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_FROM_SEED_ADDRESS_SCHEME">OBJECT_FROM_SEED_ADDRESS_SCHEME</a>);
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes))
}
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address_impl"></a>

## Function `create_user_derived_object_address_impl`



<pre><code><b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>;
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object_address"></a>

## Function `create_user_derived_object_address`

Derives an object address from the source address and an object: sha3_256([source | object addr | 0xFC]).


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address">create_user_derived_object_address</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address">create_user_derived_object_address</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b> {
    <b>if</b> (std::features::object_native_derived_address_enabled()) {
        <a href="object.md#0x1_object_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source, derive_from)
    } <b>else</b> {
        <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&source);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> bytes, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&derive_from));
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_DERIVED_SCHEME">OBJECT_DERIVED_SCHEME</a>);
        <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes))
    }
}
</code></pre>



</details>

<a id="0x1_object_create_guid_object_address"></a>

## Function `create_guid_object_address`

Derives an object from an Account GUID.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid_object_address">create_guid_object_address</a>(source: <b>address</b>, creation_num: u64): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid_object_address">create_guid_object_address</a>(source: <b>address</b>, creation_num: u64): <b>address</b> {
    <b>let</b> id = <a href="guid.md#0x1_guid_create_id">guid::create_id</a>(source, creation_num);
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&id);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>);
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes))
}
</code></pre>



</details>

<a id="0x1_object_exists_at"></a>

## Function `exists_at`



<pre><code><b>fun</b> <a href="object.md#0x1_object_exists_at">exists_at</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="object.md#0x1_object_exists_at">exists_at</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool;
</code></pre>



</details>

<a id="0x1_object_object_address"></a>

## Function `object_address`

Returns the address of within an ObjectId.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_address">object_address</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_address">object_address</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: &<a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): <b>address</b> {
    <a href="object.md#0x1_object">object</a>.inner
}
</code></pre>



</details>

<a id="0x1_object_convert"></a>

## Function `convert`

Convert Object<X> to Object<Y>.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_convert">convert</a>&lt;X: key, Y: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;X&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;Y&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_convert">convert</a>&lt;X: key, Y: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;X&gt;): <a href="object.md#0x1_object_Object">Object</a>&lt;Y&gt; {
    <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;Y&gt;(<a href="object.md#0x1_object">object</a>.inner)
}
</code></pre>



</details>

<a id="0x1_object_create_named_object"></a>

## Function `create_named_object`

Create a new named object and return the ConstructorRef. Named objects can be queried globally
by knowing the user generated seed used to create them. Named objects cannot be deleted.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_named_object">create_named_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_named_object">create_named_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> creator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> obj_addr = <a href="object.md#0x1_object_create_object_address">create_object_address</a>(&creator_address, seed);
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address, obj_addr, <b>false</b>)
}
</code></pre>



</details>

<a id="0x1_object_create_user_derived_object"></a>

## Function `create_user_derived_object`

Create a new object whose address is derived based on the creator account address and another object.
Derivde objects, similar to named objects, cannot be deleted.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object">create_user_derived_object</a>(creator_address: <b>address</b>, derive_ref: &<a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object">create_user_derived_object</a>(creator_address: <b>address</b>, derive_ref: &<a href="object.md#0x1_object_DeriveRef">DeriveRef</a>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> obj_addr = <a href="object.md#0x1_object_create_user_derived_object_address">create_user_derived_object_address</a>(creator_address, derive_ref.self);
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address, obj_addr, <b>false</b>)
}
</code></pre>



</details>

<a id="0x1_object_create_object"></a>

## Function `create_object`

Create a new object by generating a random unique address based on transaction hash.
The unique address is computed sha3_256([transaction hash | auid counter | 0xFB]).
The created object is deletable as we can guarantee the same unique address can
never be regenerated with future txs.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object">create_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object">create_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> unique_address = <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">transaction_context::generate_auid_address</a>();
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(owner_address, unique_address, <b>true</b>)
}
</code></pre>



</details>

<a id="0x1_object_create_sticky_object"></a>

## Function `create_sticky_object`

Same as <code>create_object</code> except the object to be created will be undeletable.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_sticky_object">create_sticky_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_sticky_object">create_sticky_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> unique_address = <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">transaction_context::generate_auid_address</a>();
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(owner_address, unique_address, <b>false</b>)
}
</code></pre>



</details>

<a id="0x1_object_create_sticky_object_at_address"></a>

## Function `create_sticky_object_at_address`

Create a sticky object at a specific address. Only used by aptos_framework::coin.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_sticky_object_at_address">create_sticky_object_at_address</a>(owner_address: <b>address</b>, object_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_sticky_object_at_address">create_sticky_object_at_address</a>(
    owner_address: <b>address</b>,
    object_address: <b>address</b>,
): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(owner_address, object_address, <b>false</b>)
}
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


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_account">create_object_from_account</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_account">create_object_from_account</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="account.md#0x1_account_create_guid">account::create_guid</a>(creator);
    <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), <a href="guid.md#0x1_guid">guid</a>)
}
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


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_object">create_object_from_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_object">create_object_from_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="object.md#0x1_object_create_guid">create_guid</a>(creator);
    <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), <a href="guid.md#0x1_guid">guid</a>)
}
</code></pre>



</details>

<a id="0x1_object_create_object_from_guid"></a>

## Function `create_object_from_guid`



<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(creator_address: <b>address</b>, <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(creator_address: <b>address</b>, <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="guid.md#0x1_guid">guid</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, <a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>);
    <b>let</b> obj_addr = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes));
    <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address, obj_addr, <b>true</b>)
}
</code></pre>



</details>

<a id="0x1_object_create_object_internal"></a>

## Function `create_object_internal`



<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address: <b>address</b>, <a href="object.md#0x1_object">object</a>: <b>address</b>, can_delete: bool): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(
    creator_address: <b>address</b>,
    <a href="object.md#0x1_object">object</a>: <b>address</b>,
    can_delete: bool,
): <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="object.md#0x1_object_EOBJECT_EXISTS">EOBJECT_EXISTS</a>));

    <b>let</b> object_signer = <a href="create_signer.md#0x1_create_signer">create_signer</a>(<a href="object.md#0x1_object">object</a>);
    <b>let</b> guid_creation_num = <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>;
    <b>let</b> transfer_events_guid = <a href="guid.md#0x1_guid_create">guid::create</a>(<a href="object.md#0x1_object">object</a>, &<b>mut</b> guid_creation_num);

    <b>move_to</b>(
        &object_signer,
        <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
            guid_creation_num,
            owner: creator_address,
            allow_ungated_transfer: <b>true</b>,
            transfer_events: <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(transfer_events_guid),
        },
    );
    <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: <a href="object.md#0x1_object">object</a>, can_delete }
}
</code></pre>



</details>

<a id="0x1_object_generate_delete_ref"></a>

## Function `generate_delete_ref`

Generates the DeleteRef, which can be used to remove ObjectCore from global storage.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_delete_ref">generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_delete_ref">generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> {
    <b>assert</b>!(ref.can_delete, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ECANNOT_DELETE">ECANNOT_DELETE</a>));
    <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> { self: ref.self }
}
</code></pre>



</details>

<a id="0x1_object_generate_extend_ref"></a>

## Function `generate_extend_ref`

Generates the ExtendRef, which can be used to add new events and resources to the object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_extend_ref">generate_extend_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_extend_ref">generate_extend_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> {
    <a href="object.md#0x1_object_ExtendRef">ExtendRef</a> { self: ref.self }
}
</code></pre>



</details>

<a id="0x1_object_generate_transfer_ref"></a>

## Function `generate_transfer_ref`

Generates the TransferRef, which can be used to manage object transfers.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_transfer_ref">generate_transfer_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_TransferRef">object::TransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_transfer_ref">generate_transfer_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="object.md#0x1_object_TransferRef">TransferRef</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_EOBJECT_NOT_TRANSFERRABLE">EOBJECT_NOT_TRANSFERRABLE</a>));
    <a href="object.md#0x1_object_TransferRef">TransferRef</a> { self: ref.self }
}
</code></pre>



</details>

<a id="0x1_object_generate_derive_ref"></a>

## Function `generate_derive_ref`

Generates the DeriveRef, which can be used to create determnistic derived objects from the current object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_derive_ref">generate_derive_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_derive_ref">generate_derive_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="object.md#0x1_object_DeriveRef">DeriveRef</a> {
    <a href="object.md#0x1_object_DeriveRef">DeriveRef</a> { self: ref.self }
}
</code></pre>



</details>

<a id="0x1_object_generate_signer"></a>

## Function `generate_signer`

Create a signer for the ConstructorRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer">generate_signer</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer">generate_signer</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(ref.self)
}
</code></pre>



</details>

<a id="0x1_object_address_from_constructor_ref"></a>

## Function `address_from_constructor_ref`

Returns the address associated with the constructor


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_constructor_ref">address_from_constructor_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_constructor_ref">address_from_constructor_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <b>address</b> {
    ref.self
}
</code></pre>



</details>

<a id="0x1_object_object_from_constructor_ref"></a>

## Function `object_from_constructor_ref`

Returns an Object<T> from within a ConstructorRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_constructor_ref">object_from_constructor_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_constructor_ref">object_from_constructor_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; {
    <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;T&gt;(ref.self)
}
</code></pre>



</details>

<a id="0x1_object_can_generate_delete_ref"></a>

## Function `can_generate_delete_ref`

Returns whether or not the ConstructorRef can be used to create DeleteRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_can_generate_delete_ref">can_generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_can_generate_delete_ref">can_generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>): bool {
    ref.can_delete
}
</code></pre>



</details>

<a id="0x1_object_create_guid"></a>

## Function `create_guid`

Create a guid for the object, typically used for events


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a> <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>);
    <b>let</b> object_data = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(addr);
    <a href="guid.md#0x1_guid_create">guid::create</a>(addr, &<b>mut</b> object_data.guid_creation_num)
}
</code></pre>



</details>

<a id="0x1_object_new_event_handle"></a>

## Function `new_event_handle`

Generate a new event handle.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_new_event_handle">new_event_handle</a>&lt;T: drop + store&gt;(
    <a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt; <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(<a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>))
}
</code></pre>



</details>

<a id="0x1_object_address_from_delete_ref"></a>

## Function `address_from_delete_ref`

Returns the address associated with the constructor


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_delete_ref">address_from_delete_ref</a>(ref: &<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_delete_ref">address_from_delete_ref</a>(ref: &<a href="object.md#0x1_object_DeleteRef">DeleteRef</a>): <b>address</b> {
    ref.self
}
</code></pre>



</details>

<a id="0x1_object_object_from_delete_ref"></a>

## Function `object_from_delete_ref`

Returns an Object<T> from within a DeleteRef.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_delete_ref">object_from_delete_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_delete_ref">object_from_delete_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_DeleteRef">DeleteRef</a>): <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; {
    <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;T&gt;(ref.self)
}
</code></pre>



</details>

<a id="0x1_object_delete"></a>

## Function `delete`

Removes from the specified Object from global storage.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_delete">delete</a>(ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_delete">delete</a>(ref: <a href="object.md#0x1_object_DeleteRef">DeleteRef</a>) <b>acquires</b> <a href="object.md#0x1_object_Untransferable">Untransferable</a>, <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> object_core = <b>move_from</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
    <b>let</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
        guid_creation_num: _,
        owner: _,
        allow_ungated_transfer: _,
        transfer_events,
    } = object_core;

    <b>if</b> (<b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self)) {
      <b>let</b> <a href="object.md#0x1_object_Untransferable">Untransferable</a> {} = <b>move_from</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
    };

    <a href="event.md#0x1_event_destroy_handle">event::destroy_handle</a>(transfer_events);
}
</code></pre>



</details>

<a id="0x1_object_generate_signer_for_extending"></a>

## Function `generate_signer_for_extending`

Create a signer for the ExtendRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer_for_extending">generate_signer_for_extending</a>(ref: &<a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_signer_for_extending">generate_signer_for_extending</a>(ref: &<a href="object.md#0x1_object_ExtendRef">ExtendRef</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(ref.self)
}
</code></pre>



</details>

<a id="0x1_object_address_from_extend_ref"></a>

## Function `address_from_extend_ref`

Returns an address from within a ExtendRef.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_extend_ref">address_from_extend_ref</a>(ref: &<a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_from_extend_ref">address_from_extend_ref</a>(ref: &<a href="object.md#0x1_object_ExtendRef">ExtendRef</a>): <b>address</b> {
    ref.self
}
</code></pre>



</details>

<a id="0x1_object_disable_ungated_transfer"></a>

## Function `disable_ungated_transfer`

Disable direct transfer, transfers can only be triggered via a TransferRef


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_disable_ungated_transfer">disable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_disable_ungated_transfer">disable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">TransferRef</a>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
    <a href="object.md#0x1_object">object</a>.allow_ungated_transfer = <b>false</b>;
}
</code></pre>



</details>

<a id="0x1_object_set_untransferable"></a>

## Function `set_untransferable`

Prevent moving of the object


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_set_untransferable">set_untransferable</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_set_untransferable">set_untransferable</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
    <a href="object.md#0x1_object">object</a>.allow_ungated_transfer = <b>false</b>;
    <b>let</b> object_signer = <a href="object.md#0x1_object_generate_signer">generate_signer</a>(ref);
    <b>move_to</b>(&object_signer, <a href="object.md#0x1_object_Untransferable">Untransferable</a> {});
}
</code></pre>



</details>

<a id="0x1_object_enable_ungated_transfer"></a>

## Function `enable_ungated_transfer`

Enable direct transfer.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_enable_ungated_transfer">enable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_enable_ungated_transfer">enable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">TransferRef</a>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_EOBJECT_NOT_TRANSFERRABLE">EOBJECT_NOT_TRANSFERRABLE</a>));
    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
    <a href="object.md#0x1_object">object</a>.allow_ungated_transfer = <b>true</b>;
}
</code></pre>



</details>

<a id="0x1_object_generate_linear_transfer_ref"></a>

## Function `generate_linear_transfer_ref`

Create a LinearTransferRef for a one-time transfer. This requires that the owner at the
time of generation is the owner at the time of transferring.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_linear_transfer_ref">generate_linear_transfer_ref</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>): <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_linear_transfer_ref">generate_linear_transfer_ref</a>(ref: &<a href="object.md#0x1_object_TransferRef">TransferRef</a>): <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_EOBJECT_NOT_TRANSFERRABLE">EOBJECT_NOT_TRANSFERRABLE</a>));
    <b>let</b> owner = <a href="object.md#0x1_object_owner">owner</a>(<a href="object.md#0x1_object_Object">Object</a>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt; { inner: ref.self });
    <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> {
        self: ref.self,
        owner,
    }
}
</code></pre>



</details>

<a id="0x1_object_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer to the destination address using a LinearTransferRef.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_with_ref">transfer_with_ref</a>(ref: <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_with_ref">transfer_with_ref</a>(ref: <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a>, <b>to</b>: <b>address</b>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a>, <a href="object.md#0x1_object_TombStone">TombStone</a> {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_EOBJECT_NOT_TRANSFERRABLE">EOBJECT_NOT_TRANSFERRABLE</a>));

    // Undo soft burn <b>if</b> present <b>as</b> we don't want the original owner <b>to</b> be able <b>to</b> reclaim by calling unburn later.
    <b>if</b> (<b>exists</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(ref.self)) {
        <b>let</b> <a href="object.md#0x1_object_TombStone">TombStone</a> { original_owner: _ } = <b>move_from</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(ref.self);
    };

    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
    <b>assert</b>!(
        <a href="object.md#0x1_object">object</a>.owner == ref.owner,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>),
    );
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="object.md#0x1_object_Transfer">Transfer</a> {
                <a href="object.md#0x1_object">object</a>: ref.self,
                from: <a href="object.md#0x1_object">object</a>.owner,
                <b>to</b>,
            },
        );
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="object.md#0x1_object">object</a>.transfer_events,
        <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> {
            <a href="object.md#0x1_object">object</a>: ref.self,
            from: <a href="object.md#0x1_object">object</a>.owner,
            <b>to</b>,
        },
    );
    <a href="object.md#0x1_object">object</a>.owner = <b>to</b>;
}
</code></pre>



</details>

<a id="0x1_object_transfer_call"></a>

## Function `transfer_call`

Entry function that can be used to transfer, if allow_ungated_transfer is set true.


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_call">transfer_call</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_call">transfer_call</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="object.md#0x1_object">object</a>: <b>address</b>,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <a href="object.md#0x1_object_transfer_raw">transfer_raw</a>(owner, <a href="object.md#0x1_object">object</a>, <b>to</b>)
}
</code></pre>



</details>

<a id="0x1_object_transfer"></a>

## Function `transfer`

Transfers ownership of the object (and all associated resources) at the specified address
for Object<T> to the "to" address.


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer">transfer</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer">transfer</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <a href="object.md#0x1_object_transfer_raw">transfer_raw</a>(owner, <a href="object.md#0x1_object">object</a>.inner, <b>to</b>)
}
</code></pre>



</details>

<a id="0x1_object_transfer_raw"></a>

## Function `transfer_raw`

Attempts to transfer using addresses only. Transfers the given object if
allow_ungated_transfer is set true. Note, that this allows the owner of a nested object to
transfer that object, so long as allow_ungated_transfer is enabled at each stage in the
hierarchy.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_raw">transfer_raw</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_raw">transfer_raw</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="object.md#0x1_object">object</a>: <b>address</b>,
    <b>to</b>: <b>address</b>,
) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner_address, <a href="object.md#0x1_object">object</a>);
    <a href="object.md#0x1_object_transfer_raw_inner">transfer_raw_inner</a>(<a href="object.md#0x1_object">object</a>, <b>to</b>);
}
</code></pre>



</details>

<a id="0x1_object_transfer_raw_inner"></a>

## Function `transfer_raw_inner`



<pre><code><b>fun</b> <a href="object.md#0x1_object_transfer_raw_inner">transfer_raw_inner</a>(<a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="object.md#0x1_object_transfer_raw_inner">transfer_raw_inner</a>(<a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> object_core = <b>borrow_global_mut</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
    <b>if</b> (object_core.owner != <b>to</b>) {
        <b>if</b> (std::features::module_event_migration_enabled()) {
            <a href="event.md#0x1_event_emit">event::emit</a>(
                <a href="object.md#0x1_object_Transfer">Transfer</a> {
                    <a href="object.md#0x1_object">object</a>,
                    from: object_core.owner,
                    <b>to</b>,
                },
            );
        };
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> object_core.transfer_events,
            <a href="object.md#0x1_object_TransferEvent">TransferEvent</a> {
                <a href="object.md#0x1_object">object</a>,
                from: object_core.owner,
                <b>to</b>,
            },
        );
        object_core.owner = <b>to</b>;
    };
}
</code></pre>



</details>

<a id="0x1_object_transfer_to_object"></a>

## Function `transfer_to_object`

Transfer the given object to another object. See <code>transfer</code> for more information.


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_to_object">transfer_to_object</a>&lt;O: key, T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;O&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_to_object">transfer_to_object</a>&lt;O: key, T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;O&gt;,
    <b>to</b>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;,
) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <a href="object.md#0x1_object_transfer">transfer</a>(owner, <a href="object.md#0x1_object">object</a>, <b>to</b>.inner)
}
</code></pre>



</details>

<a id="0x1_object_verify_ungated_and_descendant"></a>

## Function `verify_ungated_and_descendant`

This checks that the destination address is eventually owned by the owner and that each
object between the two allows for ungated transfers. Note, this is limited to a depth of 8
objects may have cyclic dependencies.


<pre><code><b>fun</b> <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner: <b>address</b>, destination: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner: <b>address</b>, destination: <b>address</b>) <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> current_address = destination;
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );

    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address);
    <b>assert</b>!(
        <a href="object.md#0x1_object">object</a>.allow_ungated_transfer,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>),
    );

    <b>let</b> current_address = <a href="object.md#0x1_object">object</a>.owner;
    <b>let</b> count = 0;
    <b>while</b> (owner != current_address) {
        count = count + 1;
        <b>assert</b>!(count &lt; <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="object.md#0x1_object_EMAXIMUM_NESTING">EMAXIMUM_NESTING</a>));
        // At this point, the first <a href="object.md#0x1_object">object</a> <b>exists</b> and so the more likely case is that the
        // <a href="object.md#0x1_object">object</a>'s owner is not an <a href="object.md#0x1_object">object</a>. So we <b>return</b> a more sensible <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a>.
        <b>assert</b>!(
            <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>),
        );
        <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address);
        <b>assert</b>!(
            <a href="object.md#0x1_object">object</a>.allow_ungated_transfer,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENO_UNGATED_TRANSFERS">ENO_UNGATED_TRANSFERS</a>),
        );
        current_address = <a href="object.md#0x1_object">object</a>.owner;
    };
}
</code></pre>



</details>

<a id="0x1_object_burn"></a>

## Function `burn`

Previously allowed to burn objects, has now been disabled.  Objects can still be unburnt.

Please use the test only [<code>object::burn_object</code>] for testing with previously burned objects.


<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_burn">burn</a>&lt;T: key&gt;(_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_burn">burn</a>&lt;T: key&gt;(_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _object: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;) {
    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_EBURN_NOT_ALLOWED">EBURN_NOT_ALLOWED</a>)
}
</code></pre>



</details>

<a id="0x1_object_unburn"></a>

## Function `unburn`

Allow origin owners to reclaim any objects they previous burnt.


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_unburn">unburn</a>&lt;T: key&gt;(original_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_unburn">unburn</a>&lt;T: key&gt;(
    original_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;,
) <b>acquires</b> <a href="object.md#0x1_object_TombStone">TombStone</a>, <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> object_addr = <a href="object.md#0x1_object">object</a>.inner;
    <b>assert</b>!(<b>exists</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(object_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="object.md#0x1_object_EOBJECT_NOT_BURNT">EOBJECT_NOT_BURNT</a>));

    <b>let</b> <a href="object.md#0x1_object_TombStone">TombStone</a> { original_owner: original_owner_addr } = <b>move_from</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(object_addr);
    <b>assert</b>!(original_owner_addr == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(original_owner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object.md#0x1_object_ENOT_OBJECT_OWNER">ENOT_OBJECT_OWNER</a>));
    <a href="object.md#0x1_object_transfer_raw_inner">transfer_raw_inner</a>(object_addr, original_owner_addr);
}
</code></pre>



</details>

<a id="0x1_object_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Accessors
Return true if ungated transfer is allowed.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): bool <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );
    <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner).allow_ungated_transfer
}
</code></pre>



</details>

<a id="0x1_object_owner"></a>

## Function `owner`

Return the current owner.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owner">owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owner">owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );
    <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner).owner
}
</code></pre>



</details>

<a id="0x1_object_is_owner"></a>

## Function `is_owner`

Return true if the provided address is the current owner.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_owner">is_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_owner">is_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;, owner: <b>address</b>): bool <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <a href="object.md#0x1_object_owner">owner</a>(<a href="object.md#0x1_object">object</a>) == owner
}
</code></pre>



</details>

<a id="0x1_object_owns"></a>

## Function `owns`

Return true if the provided address has indirect or direct ownership of the provided object.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owns">owns</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owns">owns</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;, owner: <b>address</b>): bool <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> current_address = <a href="object.md#0x1_object_object_address">object_address</a>(&<a href="object.md#0x1_object">object</a>);
    <b>if</b> (current_address == owner) {
        <b>return</b> <b>true</b>
    };

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object.md#0x1_object_EOBJECT_DOES_NOT_EXIST">EOBJECT_DOES_NOT_EXIST</a>),
    );

    <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address);
    <b>let</b> current_address = <a href="object.md#0x1_object">object</a>.owner;

    <b>let</b> count = 0;
    <b>while</b> (owner != current_address) {
        count = count + 1;
        <b>assert</b>!(count &lt; <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="object.md#0x1_object_EMAXIMUM_NESTING">EMAXIMUM_NESTING</a>));
        <b>if</b> (!<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address)) {
            <b>return</b> <b>false</b>
        };

        <b>let</b> <a href="object.md#0x1_object">object</a> = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address);
        current_address = <a href="object.md#0x1_object">object</a>.owner;
    };
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_object_root_owner"></a>

## Function `root_owner`

Returns the root owner of an object. As objects support nested ownership, it can be useful
to determine the identity of the starting point of ownership.


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_root_owner">root_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_root_owner">root_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    <b>let</b> obj_owner = <a href="object.md#0x1_object_owner">owner</a>(<a href="object.md#0x1_object">object</a>);
    <b>while</b> (<a href="object.md#0x1_object_is_object">is_object</a>(obj_owner)) {
        obj_owner = <a href="object.md#0x1_object_owner">owner</a>(<a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_owner));
    };
    obj_owner
}
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


<pre><code><b>pragma</b> aborts_if_is_strict;
</code></pre>




<a id="0x1_object_spec_exists_at"></a>


<pre><code><b>fun</b> <a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool;
</code></pre>




<a id="0x1_object_spec_create_object_address"></a>


<pre><code><b>fun</b> <a href="object.md#0x1_object_spec_create_object_address">spec_create_object_address</a>(source: <b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>;
</code></pre>




<a id="0x1_object_spec_create_user_derived_object_address"></a>


<pre><code><b>fun</b> <a href="object.md#0x1_object_spec_create_user_derived_object_address">spec_create_user_derived_object_address</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>;
</code></pre>




<a id="0x1_object_spec_create_guid_object_address"></a>


<pre><code><b>fun</b> <a href="object.md#0x1_object_spec_create_guid_object_address">spec_create_guid_object_address</a>(source: <b>address</b>, creation_num: u64): <b>address</b>;
</code></pre>



<a id="@Specification_1_address_to_object"></a>

### Function `address_to_object`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_address_to_object">address_to_object</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
<b>aborts_if</b> !<a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;T&gt;(<a href="object.md#0x1_object">object</a>);
<b>ensures</b> result == <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; { inner: <a href="object.md#0x1_object">object</a> };
</code></pre>



<a id="@Specification_1_create_object_address"></a>

### Function `create_object_address`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_address">create_object_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> aborts_if_is_strict = <b>false</b>;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="object.md#0x1_object_spec_create_object_address">spec_create_object_address</a>(source, seed);
</code></pre>




<a id="0x1_object_spec_create_user_derived_object_address_impl"></a>


<pre><code><b>fun</b> <a href="object.md#0x1_object_spec_create_user_derived_object_address_impl">spec_create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>;
</code></pre>



<a id="@Specification_1_create_user_derived_object_address_impl"></a>

### Function `create_user_derived_object_address_impl`


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="object.md#0x1_object_spec_create_user_derived_object_address_impl">spec_create_user_derived_object_address_impl</a>(source, derive_from);
</code></pre>



<a id="@Specification_1_create_user_derived_object_address"></a>

### Function `create_user_derived_object_address`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object_address">create_user_derived_object_address</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> aborts_if_is_strict = <b>false</b>;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="object.md#0x1_object_spec_create_user_derived_object_address">spec_create_user_derived_object_address</a>(source, derive_from);
</code></pre>



<a id="@Specification_1_create_guid_object_address"></a>

### Function `create_guid_object_address`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid_object_address">create_guid_object_address</a>(source: <b>address</b>, creation_num: u64): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> aborts_if_is_strict = <b>false</b>;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="object.md#0x1_object_spec_create_guid_object_address">spec_create_guid_object_address</a>(source, creation_num);
</code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code><b>fun</b> <a href="object.md#0x1_object_exists_at">exists_at</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <b>address</b>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;T&gt;(<a href="object.md#0x1_object">object</a>);
</code></pre>



<a id="@Specification_1_object_address"></a>

### Function `object_address`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_address">object_address</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="object.md#0x1_object">object</a>.inner;
</code></pre>



<a id="@Specification_1_convert"></a>

### Function `convert`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_convert">convert</a>&lt;X: key, Y: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;X&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;Y&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>aborts_if</b> !<a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;Y&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>ensures</b> result == <a href="object.md#0x1_object_Object">Object</a>&lt;Y&gt; { inner: <a href="object.md#0x1_object">object</a>.inner };
</code></pre>



<a id="@Specification_1_create_named_object"></a>

### Function `create_named_object`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_named_object">create_named_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>let</b> creator_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> obj_addr = <a href="object.md#0x1_object_spec_create_object_address">spec_create_object_address</a>(creator_address, seed);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: creator_address,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: obj_addr,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: obj_addr, can_delete: <b>false</b> };
</code></pre>



<a id="@Specification_1_create_user_derived_object"></a>

### Function `create_user_derived_object`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_user_derived_object">create_user_derived_object</a>(creator_address: <b>address</b>, derive_ref: &<a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>let</b> obj_addr = <a href="object.md#0x1_object_spec_create_user_derived_object_address">spec_create_user_derived_object_address</a>(creator_address, derive_ref.self);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: creator_address,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: obj_addr,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: obj_addr, can_delete: <b>false</b> };
</code></pre>



<a id="@Specification_1_create_object"></a>

### Function `create_object`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object">create_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> unique_address = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: owner_address,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: unique_address,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: unique_address, can_delete: <b>true</b> };
</code></pre>



<a id="@Specification_1_create_sticky_object"></a>

### Function `create_sticky_object`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_sticky_object">create_sticky_object</a>(owner_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> unique_address = <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">transaction_context::spec_generate_unique_address</a>();
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(unique_address) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: owner_address,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: unique_address,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: unique_address, can_delete: <b>false</b> };
</code></pre>



<a id="@Specification_1_create_sticky_object_at_address"></a>

### Function `create_sticky_object_at_address`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="object.md#0x1_object_create_sticky_object_at_address">create_sticky_object_at_address</a>(owner_address: <b>address</b>, object_address: <b>address</b>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_create_object_from_account"></a>

### Function `create_object_from_account`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_account">create_object_from_account</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
<b>let</b> object_data = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
<b>aborts_if</b> object_data.guid_creation_num + 1 &gt; MAX_U64;
<b>aborts_if</b> object_data.guid_creation_num + 1 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>let</b> creation_num = object_data.guid_creation_num;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
    id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
        creation_num,
        addr,
    }
};
<b>let</b> bytes_spec = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(<a href="guid.md#0x1_guid">guid</a>);
<b>let</b> bytes = concat(bytes_spec, vec&lt;u8&gt;(<a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>));
<b>let</b> hash_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes);
<b>let</b> obj_addr = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>ensures</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr).guid_creation_num == <b>old</b>(
    <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr)
).guid_creation_num + 1;
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: addr,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: obj_addr,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: obj_addr, can_delete: <b>true</b> };
</code></pre>



<a id="@Specification_1_create_object_from_object"></a>

### Function `create_object_from_object`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_object_from_object">create_object_from_object</a>(creator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
<b>let</b> object_data = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
<b>aborts_if</b> object_data.guid_creation_num + 1 &gt; MAX_U64;
<b>let</b> creation_num = object_data.guid_creation_num;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
    id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
        creation_num,
        addr,
    }
};
<b>let</b> bytes_spec = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(<a href="guid.md#0x1_guid">guid</a>);
<b>let</b> bytes = concat(bytes_spec, vec&lt;u8&gt;(<a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>));
<b>let</b> hash_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes);
<b>let</b> obj_addr = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(addr).guid_creation_num == <b>old</b>(<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(addr)).guid_creation_num + 1;
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: addr,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: obj_addr,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: obj_addr, can_delete: <b>true</b> };
</code></pre>



<a id="@Specification_1_create_object_from_guid"></a>

### Function `create_object_from_guid`


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_from_guid">create_object_from_guid</a>(creator_address: <b>address</b>, <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code><b>let</b> bytes_spec = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(<a href="guid.md#0x1_guid">guid</a>);
<b>let</b> bytes = concat(bytes_spec, vec&lt;u8&gt;(<a href="object.md#0x1_object_OBJECT_FROM_GUID_ADDRESS_SCHEME">OBJECT_FROM_GUID_ADDRESS_SCHEME</a>));
<b>let</b> hash_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes);
<b>let</b> obj_addr = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(hash_bytes);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(obj_addr) == <a href="object.md#0x1_object_ObjectCore">ObjectCore</a> {
    guid_creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1,
    owner: creator_address,
    allow_ungated_transfer: <b>true</b>,
    transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a> {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
            id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
                creation_num: <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a>,
                addr: obj_addr,
            }
        }
    }
};
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: obj_addr, can_delete: <b>true</b> };
</code></pre>



<a id="@Specification_1_create_object_internal"></a>

### Function `create_object_internal`


<pre><code><b>fun</b> <a href="object.md#0x1_object_create_object_internal">create_object_internal</a>(creator_address: <b>address</b>, <a href="object.md#0x1_object">object</a>: <b>address</b>, can_delete: bool): <a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>).guid_creation_num == <a href="object.md#0x1_object_INIT_GUID_CREATION_NUM">INIT_GUID_CREATION_NUM</a> + 1;
<b>ensures</b> result == <a href="object.md#0x1_object_ConstructorRef">ConstructorRef</a> { self: <a href="object.md#0x1_object">object</a>, can_delete };
</code></pre>



<a id="@Specification_1_generate_delete_ref"></a>

### Function `generate_delete_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_delete_ref">generate_delete_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>
</code></pre>




<pre><code><b>aborts_if</b> !ref.can_delete;
<b>ensures</b> result == <a href="object.md#0x1_object_DeleteRef">DeleteRef</a> { self: ref.self };
</code></pre>



<a id="@Specification_1_generate_transfer_ref"></a>

### Function `generate_transfer_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_transfer_ref">generate_transfer_ref</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_TransferRef">object::TransferRef</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>ensures</b> result == <a href="object.md#0x1_object_TransferRef">TransferRef</a> {
    self: ref.self,
};
</code></pre>



<a id="@Specification_1_object_from_constructor_ref"></a>

### Function `object_from_constructor_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_constructor_ref">object_from_constructor_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>aborts_if</b> !<a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;T&gt;(ref.self);
<b>ensures</b> result == <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; { inner: ref.self };
</code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_create_guid">create_guid</a>(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>));
<b>let</b> object_data = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>));
<b>aborts_if</b> object_data.guid_creation_num + 1 &gt; MAX_U64;
<b>ensures</b> result == <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
    id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
        creation_num: object_data.guid_creation_num,
        addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>)
    }
};
</code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="object.md#0x1_object">object</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>));
<b>let</b> object_data = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>));
<b>aborts_if</b> object_data.guid_creation_num + 1 &gt; MAX_U64;
<b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="guid.md#0x1_guid_GUID">guid::GUID</a> {
    id: <a href="guid.md#0x1_guid_ID">guid::ID</a> {
        creation_num: object_data.guid_creation_num,
        addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="object.md#0x1_object">object</a>)
    }
};
<b>ensures</b> result == <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt; {
    counter: 0,
    <a href="guid.md#0x1_guid">guid</a>,
};
</code></pre>



<a id="@Specification_1_object_from_delete_ref"></a>

### Function `object_from_delete_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_object_from_delete_ref">object_from_delete_ref</a>&lt;T: key&gt;(ref: &<a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>aborts_if</b> !<a href="object.md#0x1_object_spec_exists_at">spec_exists_at</a>&lt;T&gt;(ref.self);
<b>ensures</b> result == <a href="object.md#0x1_object_Object">Object</a>&lt;T&gt; { inner: ref.self };
</code></pre>



<a id="@Specification_1_delete"></a>

### Function `delete`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_delete">delete</a>(ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>ensures</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
</code></pre>



<a id="@Specification_1_disable_ungated_transfer"></a>

### Function `disable_ungated_transfer`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_disable_ungated_transfer">disable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self).allow_ungated_transfer == <b>false</b>;
</code></pre>



<a id="@Specification_1_set_untransferable"></a>

### Function `set_untransferable`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_set_untransferable">set_untransferable</a>(ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>ensures</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self).allow_ungated_transfer == <b>false</b>;
</code></pre>



<a id="@Specification_1_enable_ungated_transfer"></a>

### Function `enable_ungated_transfer`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_enable_ungated_transfer">enable_ungated_transfer</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>)
</code></pre>




<pre><code><b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self).allow_ungated_transfer == <b>true</b>;
</code></pre>



<a id="@Specification_1_generate_linear_transfer_ref"></a>

### Function `generate_linear_transfer_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_generate_linear_transfer_ref">generate_linear_transfer_ref</a>(ref: &<a href="object.md#0x1_object_TransferRef">object::TransferRef</a>): <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>let</b> owner = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self).owner;
<b>ensures</b> result == <a href="object.md#0x1_object_LinearTransferRef">LinearTransferRef</a> {
    self: ref.self,
    owner,
};
</code></pre>



<a id="@Specification_1_transfer_with_ref"></a>

### Function `transfer_with_ref`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_with_ref">transfer_with_ref</a>(ref: <a href="object.md#0x1_object_LinearTransferRef">object::LinearTransferRef</a>, <b>to</b>: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> <b>exists</b>&lt;<a href="object.md#0x1_object_Untransferable">Untransferable</a>&gt;(ref.self);
<b>let</b> <a href="object.md#0x1_object">object</a> = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>aborts_if</b> <a href="object.md#0x1_object">object</a>.owner != ref.owner;
<b>ensures</b> <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(ref.self).owner == <b>to</b>;
</code></pre>



<a id="@Specification_1_transfer_call"></a>

### Function `transfer_call`


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_call">transfer_call</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
<b>aborts_if</b> !<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer">transfer</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>let</b> object_address = <a href="object.md#0x1_object">object</a>.inner;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(object_address);
<b>aborts_if</b> !<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(object_address).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer_raw"></a>

### Function `transfer_raw`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_transfer_raw">transfer_raw</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <b>address</b>, <b>to</b>: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>);
<b>aborts_if</b> !<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_transfer_to_object"></a>

### Function `transfer_to_object`


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_transfer_to_object">transfer_to_object</a>&lt;O: key, T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;O&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
<b>let</b> object_address = <a href="object.md#0x1_object">object</a>.inner;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(object_address);
<b>aborts_if</b> !<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(object_address).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_verify_ungated_and_descendant"></a>

### Function `verify_ungated_and_descendant`


<pre><code><b>fun</b> <a href="object.md#0x1_object_verify_ungated_and_descendant">verify_ungated_and_descendant</a>(owner: <b>address</b>, destination: <b>address</b>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>pragma</b> unroll = <a href="object.md#0x1_object_MAXIMUM_OBJECT_NESTING">MAXIMUM_OBJECT_NESTING</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(destination);
<b>aborts_if</b> !<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(destination).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_burn">burn</a>&lt;T: key&gt;(_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>




<pre><code><b>aborts_if</b> <b>true</b>;
</code></pre>



<a id="@Specification_1_unburn"></a>

### Function `unburn`


<pre><code><b>public</b> entry <b>fun</b> <a href="object.md#0x1_object_unburn">unburn</a>&lt;T: key&gt;(original_owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> object_address = <a href="object.md#0x1_object">object</a>.inner;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(object_address);
<b>aborts_if</b> !<a href="object.md#0x1_object_is_burnt">is_burnt</a>(<a href="object.md#0x1_object">object</a>);
<b>let</b> tomb_stone = <b>borrow_global</b>&lt;<a href="object.md#0x1_object_TombStone">TombStone</a>&gt;(object_address);
<b>aborts_if</b> tomb_stone.original_owner != <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(original_owner);
</code></pre>



<a id="@Specification_1_ungated_transfer_allowed"></a>

### Function `ungated_transfer_allowed`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>ensures</b> result == <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner).allow_ungated_transfer;
</code></pre>



<a id="@Specification_1_owner"></a>

### Function `owner`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owner">owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>ensures</b> result == <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner).owner;
</code></pre>



<a id="@Specification_1_is_owner"></a>

### Function `is_owner`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_is_owner">is_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, owner: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>ensures</b> result == (<b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner).owner == owner);
</code></pre>



<a id="@Specification_1_owns"></a>

### Function `owns`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_owns">owns</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, owner: <b>address</b>): bool
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> current_address_0 = <a href="object.md#0x1_object">object</a>.inner;
<b>let</b> object_0 = <b>global</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(current_address_0);
<b>let</b> current_address = object_0.owner;
<b>aborts_if</b> <a href="object.md#0x1_object">object</a>.inner != owner && !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">ObjectCore</a>&gt;(<a href="object.md#0x1_object">object</a>.inner);
<b>ensures</b> current_address_0 == owner ==&gt; result == <b>true</b>;
</code></pre>



<a id="@Specification_1_root_owner"></a>

### Function `root_owner`


<pre><code><b>public</b> <b>fun</b> <a href="object.md#0x1_object_root_owner">root_owner</a>&lt;T: key&gt;(<a href="object.md#0x1_object">object</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
