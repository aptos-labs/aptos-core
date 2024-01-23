
<a id="0x1_object_code_deployment"></a>

# Module `0x1::object_code_deployment`



-  [Resource `PublisherRef`](#0x1_object_code_deployment_PublisherRef)
-  [Constants](#@Constants_0)
-  [Function `publish`](#0x1_object_code_deployment_publish)
-  [Function `object_seed`](#0x1_object_code_deployment_object_seed)
-  [Function `upgrade`](#0x1_object_code_deployment_upgrade)
-  [Function `freeze_package_registry`](#0x1_object_code_deployment_freeze_package_registry)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="code.md#0x1_code">0x1::code</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_object_code_deployment_PublisherRef"></a>

## Resource `PublisherRef`



<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>extend_ref: <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_object_code_deployment_ENOT_OWNER"></a>

Not the owner of the <code><a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a></code>


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_ENOT_OWNER">ENOT_OWNER</a>: u64 = 2;
</code></pre>



<a id="0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED"></a>

Object code deployment not supported.


<pre><code><b>const</b> <a href="object_code_deployment.md#0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED">EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED</a>: u64 = 1;
</code></pre>



<a id="0x1_object_code_deployment_publish"></a>

## Function `publish`

Create a new object to host the code, and <code><a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a></code> if the code is upgradeable,
Send <code><a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a></code> to object signer.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_publish">publish</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_publish">publish</a>(
    publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_object_code_deployment_enabled">features::is_object_code_deployment_enabled</a>(), <a href="object_code_deployment.md#0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED">EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED</a>);

    <b>let</b> object_seed = <a href="object_code_deployment.md#0x1_object_code_deployment_object_seed">object_seed</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(publisher));
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_named_object">object::create_named_object</a>(publisher, object_seed);
    <b>let</b> module_signer = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <a href="code.md#0x1_code_publish_package_txn">code::publish_package_txn</a>(module_signer, metadata_serialized, <a href="code.md#0x1_code">code</a>);

    <b>if</b> (<a href="code.md#0x1_code_is_package_upgradeable">code::is_package_upgradeable</a>(metadata_serialized)) {
        <b>move_to</b>(module_signer, <a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a> {
            extend_ref: <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(constructor_ref)
        });
    };
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
    <b>let</b> object_seed = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"aptos_framework::object_code_deployment");
    <b>let</b> sequence_number = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(publisher) + 1;
    <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(object_seed, <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&sequence_number)));
    *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(object_seed)
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_upgrade"></a>

## Function `upgrade`

Upgrade the code in an existing code object.
Requires the publisher to be the owner of the <code><a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a></code> object.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_upgrade">upgrade</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, publisher_ref: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">object_code_deployment::PublisherRef</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_upgrade">upgrade</a>(
    publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    publisher_ref: Object&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a>&gt;,
) <b>acquires</b> <a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_object_code_deployment_enabled">features::is_object_code_deployment_enabled</a>(), <a href="object_code_deployment.md#0x1_object_code_deployment_EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED">EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED</a>);
    <b>assert</b>!(<a href="object.md#0x1_object_is_owner">object::is_owner</a>(publisher_ref, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(publisher)), <a href="object_code_deployment.md#0x1_object_code_deployment_ENOT_OWNER">ENOT_OWNER</a>);

    <b>let</b> extend_ref = &<b>borrow_global</b>&lt;<a href="object_code_deployment.md#0x1_object_code_deployment_PublisherRef">PublisherRef</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&publisher_ref)).extend_ref;
    <b>let</b> code_signer = &<a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(extend_ref);
    <a href="code.md#0x1_code_publish_package_txn">code::publish_package_txn</a>(code_signer, metadata_serialized, <a href="code.md#0x1_code">code</a>);
}
</code></pre>



</details>

<a id="0x1_object_code_deployment_freeze_package_registry"></a>

## Function `freeze_package_registry`

Make an existing upgradable package immutable.
Requires the <code>publisher</code> to be the owner of the <code>package_registry</code> object.


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_freeze_package_registry">freeze_package_registry</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, package_registry: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="object_code_deployment.md#0x1_object_code_deployment_freeze_package_registry">freeze_package_registry</a>(publisher: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, package_registry: Object&lt;PackageRegistry&gt;) {
    <a href="code.md#0x1_code_freeze_package_registry">code::freeze_package_registry</a>(publisher, package_registry);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
