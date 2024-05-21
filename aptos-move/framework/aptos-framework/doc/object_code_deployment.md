
<a id="0x1_object_code_deployment"></a>

# Module `0x1::object_code_deployment`

This module allows users to deploy, upgrade and freeze modules deployed to objects on-chain.
This enables users to deploy modules to an object with a unique address each time they are published.
This modules provides an alternative method to publish code on-chain, where code is deployed to objects rather than accounts.
This is encouraged as it abstracts the necessary resources needed for deploying modules,
along with the required authorization to upgrade and freeze modules.

The functionalities of this module are as follows.

Publishing modules flow:
1. Create a new object with the address derived from the publisher address and the object seed.
2. Publish the module passed in the function via <code>metadata_serialized</code> and <code>code</code> to the newly created object.
3. Emits 'Publish' event with the address of the newly created object.
4. Create a <code>ManagingRefs</code> which stores the extend ref of the newly created object.
Note: This is needed to upgrade the code as the signer must be generated to upgrade the existing code in an object.

Upgrading modules flow:
1. Assert the <code>code_object</code> passed in the function is owned by the <code>publisher</code>.
2. Assert the <code>code_object</code> passed in the function exists in global storage.
2. Retrieve the <code>ExtendRef</code> from the <code>code_object</code> and generate the signer from this.
3. Upgrade the module with the <code>metadata_serialized</code> and <code>code</code> passed in the function.
4. Emits 'Upgrade' event with the address of the object with the upgraded code.
Note: If the modules were deployed as immutable when calling <code>publish</code>, the upgrade will fail.

Freezing modules flow:
1. Assert the <code>code_object</code> passed in the function exists in global storage.
2. Assert the <code>code_object</code> passed in the function is owned by the <code>publisher</code>.
3. Mark all the modules in the <code>code_object</code> as immutable.
4. Emits 'Freeze' event with the address of the object with the frozen code.
Note: There is no unfreeze function as this gives no benefit if the user can freeze/unfreeze modules at will.
Once modules are marked as immutable, they cannot be made mutable again.


-  [Resource `ManagingRefs`](#0x1_object_code_deployment_ManagingRefs)
-  [Struct `Publish`](#0x1_object_code_deployment_Publish)
-  [Struct `Upgrade`](#0x1_object_code_deployment_Upgrade)
-  [Struct `Freeze`](#0x1_object_code_deployment_Freeze)
-  [Constants](#@Constants_0)
-  [Function `publish`](#0x1_object_code_deployment_publish)
-  [Function `object_seed`](#0x1_object_code_deployment_object_seed)
-  [Function `upgrade`](#0x1_object_code_deployment_upgrade)
-  [Function `freeze_code_object`](#0x1_object_code_deployment_freeze_code_object)


<pre><code>use 0x1::account;<br/>use 0x1::bcs;<br/>use 0x1::code;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::object;<br/>use 0x1::signer;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_object_code_deployment_ManagingRefs"></a>

## Resource `ManagingRefs`

Internal struct, attached to the object, that holds Refs we need to manage the code deployment (i.e. upgrades).


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct ManagingRefs has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>extend_ref: object::ExtendRef</code>
</dt>
<dd>
 We need to keep the extend ref to be able to generate the signer to upgrade existing code.
</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Publish"></a>

## Struct `Publish`

Event emitted when code is published to an object.


<pre><code>&#35;[event]<br/>struct Publish has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Upgrade"></a>

## Struct `Upgrade`

Event emitted when code in an existing object is upgraded.


<pre><code>&#35;[event]<br/>struct Upgrade has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Freeze"></a>

## Struct `Freeze`

Event emitted when code in an existing object is made immutable.


<pre><code>&#35;[event]<br/>struct Freeze has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_object_code_deployment_ECODE_OBJECT_DOES_NOT_EXIST"></a>

<code>code_object</code> does not exist.


<pre><code>const ECODE_OBJECT_DOES_NOT_EXIST: u64 &#61; 3;<br/></code></pre>



<a id="0x1_object_code_deployment_ENOT_CODE_OBJECT_OWNER"></a>

Not the owner of the <code>code_object</code>


<pre><code>const ENOT_CODE_OBJECT_OWNER: u64 &#61; 2;<br/></code></pre>



<a id="0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED"></a>

Object code deployment feature not supported.


<pre><code>const EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_object_code_deployment_OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR"></a>



<pre><code>const OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 111, 98, 106, 101, 99, 116, 95, 99, 111, 100, 101, 95, 100, 101, 112, 108, 111, 121, 109, 101, 110, 116];<br/></code></pre>



<a id="0x1_object_code_deployment_publish"></a>

## Function `publish`

Creates a new object with a unique address derived from the publisher address and the object seed.
Publishes the code passed in the function to the newly created object.
The caller must provide package metadata describing the package via <code>metadata_serialized</code> and
the code to be published via <code>code</code>. This contains a vector of modules to be deployed on-chain.


<pre><code>public entry fun publish(publisher: &amp;signer, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun publish(<br/>    publisher: &amp;signer,<br/>    metadata_serialized: vector&lt;u8&gt;,<br/>    code: vector&lt;vector&lt;u8&gt;&gt;,<br/>) &#123;<br/>    assert!(<br/>        features::is_object_code_deployment_enabled(),<br/>        error::unavailable(EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED),<br/>    );<br/><br/>    let publisher_address &#61; signer::address_of(publisher);<br/>    let object_seed &#61; object_seed(publisher_address);<br/>    let constructor_ref &#61; &amp;object::create_named_object(publisher, object_seed);<br/>    let code_signer &#61; &amp;object::generate_signer(constructor_ref);<br/>    code::publish_package_txn(code_signer, metadata_serialized, code);<br/><br/>    event::emit(Publish &#123; object_address: signer::address_of(code_signer), &#125;);<br/><br/>    move_to(code_signer, ManagingRefs &#123;<br/>        extend_ref: object::generate_extend_ref(constructor_ref),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_code_deployment_object_seed"></a>

## Function `object_seed`



<pre><code>fun object_seed(publisher: address): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun object_seed(publisher: address): vector&lt;u8&gt; &#123;<br/>    let sequence_number &#61; account::get_sequence_number(publisher) &#43; 1;<br/>    let seeds &#61; vector[];<br/>    vector::append(&amp;mut seeds, bcs::to_bytes(&amp;OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR));<br/>    vector::append(&amp;mut seeds, bcs::to_bytes(&amp;sequence_number));<br/>    seeds<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_code_deployment_upgrade"></a>

## Function `upgrade`

Upgrades the existing modules at the <code>code_object</code> address with the new modules passed in <code>code</code>,
along with the metadata <code>metadata_serialized</code>.
Note: If the modules were deployed as immutable when calling <code>publish</code>, the upgrade will fail.
Requires the publisher to be the owner of the <code>code_object</code>.


<pre><code>public entry fun upgrade(publisher: &amp;signer, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;, code_object: object::Object&lt;code::PackageRegistry&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun upgrade(<br/>    publisher: &amp;signer,<br/>    metadata_serialized: vector&lt;u8&gt;,<br/>    code: vector&lt;vector&lt;u8&gt;&gt;,<br/>    code_object: Object&lt;PackageRegistry&gt;,<br/>) acquires ManagingRefs &#123;<br/>    let publisher_address &#61; signer::address_of(publisher);<br/>    assert!(<br/>        object::is_owner(code_object, publisher_address),<br/>        error::permission_denied(ENOT_CODE_OBJECT_OWNER),<br/>    );<br/><br/>    let code_object_address &#61; object::object_address(&amp;code_object);<br/>    assert!(exists&lt;ManagingRefs&gt;(code_object_address), error::not_found(ECODE_OBJECT_DOES_NOT_EXIST));<br/><br/>    let extend_ref &#61; &amp;borrow_global&lt;ManagingRefs&gt;(code_object_address).extend_ref;<br/>    let code_signer &#61; &amp;object::generate_signer_for_extending(extend_ref);<br/>    code::publish_package_txn(code_signer, metadata_serialized, code);<br/><br/>    event::emit(Upgrade &#123; object_address: signer::address_of(code_signer), &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_object_code_deployment_freeze_code_object"></a>

## Function `freeze_code_object`

Make an existing upgradable package immutable. Once this is called, the package cannot be made upgradable again.
Each <code>code_object</code> should only have one package, as one package is deployed per object in this module.
Requires the <code>publisher</code> to be the owner of the <code>code_object</code>.


<pre><code>public entry fun freeze_code_object(publisher: &amp;signer, code_object: object::Object&lt;code::PackageRegistry&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun freeze_code_object(publisher: &amp;signer, code_object: Object&lt;PackageRegistry&gt;) &#123;<br/>    code::freeze_code_object(publisher, code_object);<br/><br/>    event::emit(Freeze &#123; object_address: object::object_address(&amp;code_object), &#125;);<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
