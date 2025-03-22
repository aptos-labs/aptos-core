
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
2. Publish the module passed in the function via <code>metadata_serialized</code> and <code><a href="code.md#0x1_code">code</a></code> to the newly created object.
3. Emits 'Publish' event with the address of the newly created object.
4. Create a <code><a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a></code> which stores the extend ref of the newly created object.
Note: This is needed to upgrade the code as the signer must be generated to upgrade the existing code in an object.

Upgrading modules flow:
1. Assert the <code>code_object</code> passed in the function is owned by the <code>publisher</code>.
2. Assert the <code>code_object</code> passed in the function exists in global storage.
2. Retrieve the <code>ExtendRef</code> from the <code>code_object</code> and generate the signer from this.
3. Upgrade the module with the <code>metadata_serialized</code> and <code><a href="code.md#0x1_code">code</a></code> passed in the function.
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
-  [Resource `CodeSignerCapability`](#0x1_object_code_deployment_CodeSignerCapability)
-  [Struct `Publish`](#0x1_object_code_deployment_Publish)
-  [Struct `Upgrade`](#0x1_object_code_deployment_Upgrade)
-  [Struct `Freeze`](#0x1_object_code_deployment_Freeze)
-  [Constants](#@Constants_0)
-  [Function `next_code_object_address`](#0x1_object_code_deployment_next_code_object_address)
-  [Function `publish`](#0x1_object_code_deployment_publish)
-  [Function `object_seed`](#0x1_object_code_deployment_object_seed)
-  [Function `upgrade`](#0x1_object_code_deployment_upgrade)
-  [Function `freeze_code_object`](#0x1_object_code_deployment_freeze_code_object)
-  [Function `register_signer_capability_proof`](#0x1_object_code_deployment_register_signer_capability_proof)
-  [Function `generate_signer`](#0x1_object_code_deployment_generate_signer)
-  [Function `assert_is_code_object`](#0x1_object_code_deployment_assert_is_code_object)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="code.md#0x1_code">0x1::code</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_object_code_deployment_ManagingRefs"></a>

## Resource `ManagingRefs`

Internal struct, attached to the object, that holds Refs we need to manage the code deployment (i.e. upgrades).


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>extend_ref: <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>
 We need to keep the extend ref to be able to generate the signer to upgrade existing code.
</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_CodeSignerCapability"></a>

## Resource `CodeSignerCapability`

Allow access to the code object's signer based on a struct-based registered proof.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>capability_proof: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Publish"></a>

## Struct `Publish`

Event emitted when code is published to an object.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_Publish">Publish</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Upgrade"></a>

## Struct `Upgrade`

Event emitted when code in an existing object is upgraded.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_Upgrade">Upgrade</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_object_code_deployment_Freeze"></a>

## Struct `Freeze`

Event emitted when code in an existing object is made immutable.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_Freeze">Freeze</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>object_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_object_code_deployment_ECODE_OBJECT_DOES_NOT_EXIST"></a>

<code>code_object</code> does not exist.


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ECODE_OBJECT_DOES_NOT_EXIST">ECODE_OBJECT_DOES_NOT_EXIST</a>: u64 = 3;
</code></pre>



<a id="0x1_object_code_deployment_ENO_CODE_PERMISSION"></a>

Current permissioned signer cannot deploy object code.


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ENO_CODE_PERMISSION">ENO_CODE_PERMISSION</a>: u64 = 4;
</code></pre>



<a id="0x1_object_code_deployment_ENOT_CODE_OBJECT_OWNER"></a>

Not the owner of the <code>code_object</code>


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ENOT_CODE_OBJECT_OWNER">ENOT_CODE_OBJECT_OWNER</a>: u64 = 2;
</code></pre>



<a id="0x1_object_code_deployment_ENO_SIGNER_CAPABILITY_CONFIGURED"></a>

No signer capability proof configured for this code object.


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ENO_SIGNER_CAPABILITY_CONFIGURED">ENO_SIGNER_CAPABILITY_CONFIGURED</a>: u64 = 5;
</code></pre>



<a id="0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED"></a>

Object code deployment feature not supported.


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED">EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED</a>: u64 = 1;
</code></pre>



<a id="0x1_object_code_deployment_OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR"></a>



<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR">OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 111, 98, 106, 101, 99, 116, 95, 99, 111, 100, 101, 95, 100, 101, 112, 108, 111, 121, 109, 101, 110, 116];
</code></pre>



<a id="0x1_object_code_deployment_next_code_object_address"></a>

## Function `next_code_object_address`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_next_code_object_address">next_code_object_address</a>(publisher: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_next_code_object_address">next_code_object_address</a>(publisher: <b>address</b>): <b>address</b> {
    <b>let</b> object_seed = <a href="object_code_deployment.md#0x1_object_code_deployment_object_seed">object_seed</a>(publisher);
    <a href="object.md#0x1_object_create_object_address">object::create_object_address</a>(&publisher, object_seed)
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_publish"></a>

## Function `publish`

Creates a new object with a unique address derived from the publisher address and the object seed.
Publishes the code passed in the function to the newly created object.
The caller must provide package metadata describing the package via <code>metadata_serialized</code> and
the code to be published via <code><a href="code.md#0x1_code">code</a></code>. This contains a vector of modules to be deployed on-chain.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_publish">publish</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_publish">publish</a>(
    publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) {
    <a href="code.md#0x1_code_check_code_publishing_permission">code::check_code_publishing_permission</a>(publisher);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_object_code_deployment_enabled">features::is_object_code_deployment_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED">EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED</a>),
    );

    <b>let</b> publisher_address = <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(publisher);
    <b>let</b> object_seed = <a href="object_code_deployment.md#0x1_object_code_deployment_object_seed">object_seed</a>(publisher_address);
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_named_object">object::create_named_object</a>(publisher, object_seed);
    <b>let</b> code_signer = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <a href="code.md#0x1_code_publish_package_txn">code::publish_package_txn</a>(code_signer, metadata_serialized, <a href="code.md#0x1_code">code</a>);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_Publish">Publish</a> { object_address: <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(code_signer), });

    <b>move_to</b>(code_signer, <a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a> {
        extend_ref: <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(constructor_ref),
    });
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_object_seed"></a>

## Function `object_seed`



<pre><code><b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_object_seed">object_seed</a>(publisher: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_object_seed">object_seed</a>(publisher: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> sequence_number = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(publisher) + 1;
    <b>let</b> seeds = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    seeds.append(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="object_code_deployment.md#0x1_object_code_deployment_OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR">OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR</a>));
    seeds.append(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&sequence_number));
    seeds
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_upgrade"></a>

## Function `upgrade`

Upgrades the existing modules at the <code>code_object</code> address with the new modules passed in <code><a href="code.md#0x1_code">code</a></code>,
along with the metadata <code>metadata_serialized</code>.
Note: If the modules were deployed as immutable when calling <code>publish</code>, the upgrade will fail.
Requires the publisher to be the owner of the <code>code_object</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_upgrade">upgrade</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, code_object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_upgrade">upgrade</a>(
    publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    code_object: Object&lt;PackageRegistry&gt;,
) <b>acquires</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a> {
    <a href="code.md#0x1_code_check_code_publishing_permission">code::check_code_publishing_permission</a>(publisher);
    <b>let</b> publisher_address = <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(publisher);
    <b>assert</b>!(
        <a href="object.md#0x1_object_is_owner">object::is_owner</a>(code_object, publisher_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_ENOT_CODE_OBJECT_OWNER">ENOT_CODE_OBJECT_OWNER</a>),
    );

    <b>let</b> code_object_address = <a href="object.md#0x1_object_object_address">object::object_address</a>(&code_object);
    <a href="object_code_deployment.md#0x1_object_code_deployment_assert_is_code_object">assert_is_code_object</a>(code_object_address);

    <b>let</b> extend_ref = &<b>borrow_global</b>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a>&gt;(code_object_address).extend_ref;
    <b>let</b> code_signer = &<a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(extend_ref);
    <a href="code.md#0x1_code_publish_package_txn">code::publish_package_txn</a>(code_signer, metadata_serialized, <a href="code.md#0x1_code">code</a>);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_Upgrade">Upgrade</a> { object_address: <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(code_signer), });
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_freeze_code_object"></a>

## Function `freeze_code_object`

Make an existing upgradable package immutable. Once this is called, the package cannot be made upgradable again.
Each <code>code_object</code> should only have one package, as one package is deployed per object in this module.
Requires the <code>publisher</code> to be the owner of the <code>code_object</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_freeze_code_object">freeze_code_object</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, code_object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_freeze_code_object">freeze_code_object</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, code_object: Object&lt;PackageRegistry&gt;) {
    <a href="code.md#0x1_code_freeze_code_object">code::freeze_code_object</a>(publisher, code_object);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_Freeze">Freeze</a> { object_address: <a href="object.md#0x1_object_object_address">object::object_address</a>(&code_object), });
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_register_signer_capability_proof"></a>

## Function `register_signer_capability_proof`

Registers a capability proof for the <code>code_object</code> to allow generating the signer for the <code>code_object</code> later via
<code><a href="object_code_deployment.md#0x1_object_code_deployment_generate_signer">object_code_deployment::generate_signer</a></code>.

This can only be called by the owner of the <code>code_object</code> or the package itself.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_register_signer_capability_proof">register_signer_capability_proof</a>&lt;ProofType&gt;(owner_or_package: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_register_signer_capability_proof">register_signer_capability_proof</a>&lt;ProofType&gt;(
    owner_or_package: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
) <b>acquires</b> <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>, <a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a> {
    <a href="code.md#0x1_code_check_code_publishing_permission">code::check_code_publishing_permission</a>(owner_or_package);

    <b>let</b> proof_type = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProofType&gt;();
    <b>let</b> code_object_address = proof_type.account_address();
    // Disallow registering a <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> proof for an <a href="object.md#0x1_object">object</a> that is not a <a href="code.md#0x1_code">code</a> <a href="object.md#0x1_object">object</a>.
    <a href="object_code_deployment.md#0x1_object_code_deployment_assert_is_code_object">assert_is_code_object</a>(code_object_address);

    <b>let</b> caller_addr = <a href="permissioned_signer.md#0x1_permissioned_signer_address_of">permissioned_signer::address_of</a>(owner_or_package);
    <b>let</b> is_code_object_owner =
        <a href="object.md#0x1_object_is_owner">object::is_owner</a>(<a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;PackageRegistry&gt;(code_object_address), caller_addr);
    <b>let</b> is_package_itself = caller_addr == code_object_address;
    <b>assert</b>!(
        is_code_object_owner || is_package_itself,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_ENOT_CODE_OBJECT_OWNER">ENOT_CODE_OBJECT_OWNER</a>),
    );

    <b>if</b> (!<b>exists</b>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>&gt;(code_object_address)) {
        <b>let</b> code_object_signer = &<a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&<a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a>[code_object_address].extend_ref);
        <b>move_to</b>(code_object_signer, <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a> { capability_proof: proof_type });
    } <b>else</b> {
        <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>[code_object_address].capability_proof = proof_type;
    };
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_generate_signer"></a>

## Function `generate_signer`

Generates a signer for the <code>code_object</code> if the caller has registered a capability proof for it.


<pre><code><b>public</b> <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_generate_signer">generate_signer</a>&lt;ProofType&gt;(_proof: &ProofType): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_generate_signer">generate_signer</a>&lt;ProofType&gt;(
    _proof: &ProofType
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>, <a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a> {
    <b>let</b> proof_type = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProofType&gt;();
    <b>let</b> code_object_address = proof_type.account_address();
    // This is redundant <b>with</b> the check in `register_signer_capability_proof`, but we want <b>to</b> cautious here and also
    // fail early <b>if</b> the `code_object` is not a <a href="code.md#0x1_code">code</a> <a href="object.md#0x1_object">object</a>.
    <a href="object_code_deployment.md#0x1_object_code_deployment_assert_is_code_object">assert_is_code_object</a>(code_object_address);

    <b>assert</b>!(<b>exists</b>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>&gt;(code_object_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_ENO_SIGNER_CAPABILITY_CONFIGURED">ENO_SIGNER_CAPABILITY_CONFIGURED</a>));
    <b>let</b> proof_required = <a href="object_code_deployment.md#0x1_object_code_deployment_CodeSignerCapability">CodeSignerCapability</a>[code_object_address].capability_proof;
    <b>assert</b>!(proof_type == proof_required, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_ENO_CODE_PERMISSION">ENO_CODE_PERMISSION</a>));

    <a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&<a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a>[code_object_address].extend_ref)
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_assert_is_code_object"></a>

## Function `assert_is_code_object`



<pre><code><b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_assert_is_code_object">assert_is_code_object</a>(code_object: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_assert_is_code_object">assert_is_code_object</a>(code_object: <b>address</b>) {
    <b>assert</b>!(<b>exists</b>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_ManagingRefs">ManagingRefs</a>&gt;(code_object), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="object_code_deployment.md#0x1_object_code_deployment_ECODE_OBJECT_DOES_NOT_EXIST">ECODE_OBJECT_DOES_NOT_EXIST</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
