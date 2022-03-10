
<a name="0x1_TransactionPublishingOption"></a>

# Module `0x1::TransactionPublishingOption`

This module defines a struct storing the publishing policies for the VM.


-  [Resource `ChainMarker`](#0x1_TransactionPublishingOption_ChainMarker)
-  [Resource `TransactionPublishingOption`](#0x1_TransactionPublishingOption_TransactionPublishingOption)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_TransactionPublishingOption_initialize)
-  [Function `is_script_allowed`](#0x1_TransactionPublishingOption_is_script_allowed)
-  [Function `is_module_allowed`](#0x1_TransactionPublishingOption_is_module_allowed)
-  [Function `set_module_publishing_allowed`](#0x1_TransactionPublishingOption_set_module_publishing_allowed)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="Timestamp.md#0x1_Timestamp">0x1::Timestamp</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_TransactionPublishingOption_ChainMarker"></a>

## Resource `ChainMarker`



<pre><code><b>struct</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ChainMarker">ChainMarker</a>&lt;T&gt; <b>has</b> key
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

<a name="0x1_TransactionPublishingOption_TransactionPublishingOption"></a>

## Resource `TransactionPublishingOption`

Defines and holds the publishing policies for the VM. There are three possible configurations:
1. No module publishing, only allow-listed scripts are allowed.
2. No module publishing, custom scripts are allowed.
3. Both module publishing and custom scripts are allowed.
We represent these as the following resource.


<pre><code><b>struct</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>script_allow_list: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Only script hashes in the following list can be executed by the network. If the vector is empty, no
 limitation would be enforced.
</dd>
<dt>
<code>module_publishing_allowed: bool</code>
</dt>
<dd>
 Anyone can publish new module if this flag is set to true.
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_TransactionPublishingOption_ECONFIG"></a>



<pre><code><b>const</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ECONFIG">ECONFIG</a>: u64 = 1;
</code></pre>



<a name="0x1_TransactionPublishingOption_ECHAIN_MARKER"></a>



<pre><code><b>const</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ECHAIN_MARKER">ECHAIN_MARKER</a>: u64 = 0;
</code></pre>



<a name="0x1_TransactionPublishingOption_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_initialize">initialize</a>&lt;T&gt;(core_resource_account: &signer, script_allow_list: vector&lt;vector&lt;u8&gt;&gt;, module_publishing_allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_initialize">initialize</a>&lt;T&gt;(
    core_resource_account: &signer,
    script_allow_list: vector&lt;vector&lt;u8&gt;&gt;,
    module_publishing_allowed: bool,
) {
    <a href="Timestamp.md#0x1_Timestamp_assert_genesis">Timestamp::assert_genesis</a>();
    <a href="SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(core_resource_account);
    <b>assert</b>!(!<b>exists</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ChainMarker">ChainMarker</a>&lt;T&gt;&gt;(@CoreResources), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ECHAIN_MARKER">ECHAIN_MARKER</a>));
    <b>assert</b>!(!<b>exists</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a>&gt;(@CoreResources), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ECONFIG">ECONFIG</a>));

    <b>move_to</b>(core_resource_account, <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ChainMarker">ChainMarker</a>&lt;T&gt; {});
    <b>move_to</b>(
        core_resource_account,
        <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a>{
            script_allow_list,
            module_publishing_allowed
        }
    );
}
</code></pre>



</details>

<a name="0x1_TransactionPublishingOption_is_script_allowed"></a>

## Function `is_script_allowed`



<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_is_script_allowed">is_script_allowed</a>(script_hash: &vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_is_script_allowed">is_script_allowed</a>(script_hash: &vector&lt;u8&gt;): bool <b>acquires</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a> {
    <b>if</b> (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_is_empty">Vector::is_empty</a>(script_hash)) <b>return</b> <b>true</b>;
    <b>let</b> publish_option = <b>borrow_global</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a>&gt;(@CoreResources);
    // allowlist empty = open publishing, anyone can send txes
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_is_empty">Vector::is_empty</a>(&publish_option.script_allow_list)
    || <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_contains">Vector::contains</a>(&publish_option.script_allow_list, script_hash)
}
</code></pre>



</details>

<a name="0x1_TransactionPublishingOption_is_module_allowed"></a>

## Function `is_module_allowed`



<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_is_module_allowed">is_module_allowed</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_is_module_allowed">is_module_allowed</a>(): bool <b>acquires</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a> {
    <b>let</b> publish_option = <b>borrow_global</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a>&gt;(@CoreResources);

    publish_option.module_publishing_allowed
}
</code></pre>



</details>

<a name="0x1_TransactionPublishingOption_set_module_publishing_allowed"></a>

## Function `set_module_publishing_allowed`



<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_set_module_publishing_allowed">set_module_publishing_allowed</a>&lt;T&gt;(is_allowed: bool, _witness: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_set_module_publishing_allowed">set_module_publishing_allowed</a>&lt;T&gt;(is_allowed: bool, _witness: Cap&lt;T&gt;) <b>acquires</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ChainMarker">ChainMarker</a>&lt;T&gt;&gt;(@CoreResources), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_ECHAIN_MARKER">ECHAIN_MARKER</a>));
    <b>let</b> publish_option = <b>borrow_global_mut</b>&lt;<a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a>&gt;(@CoreResources);
    publish_option.module_publishing_allowed = is_allowed;

    <a href="Reconfiguration.md#0x1_Reconfiguration_reconfigure">Reconfiguration::reconfigure</a>();
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
