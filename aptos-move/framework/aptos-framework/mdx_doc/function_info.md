
<a id="0x1_function_info"></a>

# Module `0x1::function_info`

The <code><a href="function_info.md#0x1_function_info">function_info</a></code> module defines the <code><a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a></code> type which simulates a function pointer.


-  [Struct `FunctionInfo`](#0x1_function_info_FunctionInfo)
-  [Constants](#@Constants_0)
-  [Function `new_function_info`](#0x1_function_info_new_function_info)
-  [Function `new_function_info_from_address`](#0x1_function_info_new_function_info_from_address)
-  [Function `check_dispatch_type_compatibility`](#0x1_function_info_check_dispatch_type_compatibility)
-  [Function `load_module_from_function`](#0x1_function_info_load_module_from_function)
-  [Function `check_dispatch_type_compatibility_impl`](#0x1_function_info_check_dispatch_type_compatibility_impl)
-  [Function `is_identifier`](#0x1_function_info_is_identifier)
-  [Function `load_function_impl`](#0x1_function_info_load_function_impl)
-  [Specification](#@Specification_1)
    -  [Function `check_dispatch_type_compatibility_impl`](#@Specification_1_check_dispatch_type_compatibility_impl)
    -  [Function `load_function_impl`](#@Specification_1_load_function_impl)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /></code></pre>



<a id="0x1_function_info_FunctionInfo"></a>

## Struct `FunctionInfo`

A <code>String</code> holds a sequence of bytes which is guaranteed to be in utf8 format.


<pre><code><b>struct</b> <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>module_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_function_info_EINVALID_FUNCTION"></a>

Function specified in the FunctionInfo doesn&apos;t exist on chain.


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_EINVALID_FUNCTION">EINVALID_FUNCTION</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_function_info_EINVALID_IDENTIFIER"></a>

String is not a valid Move identifier


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_function_info_ENOT_ACTIVATED"></a>

Feature hasn&apos;t been activated yet.


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_ENOT_ACTIVATED">ENOT_ACTIVATED</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_function_info_new_function_info"></a>

## Function `new_function_info`

Creates a new function info from names.


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(module_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(<br />    module_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    module_name: String,<br />    function_name: String,<br />): <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> &#123;<br />    <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(module_signer),<br />        module_name,<br />        function_name,<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_function_info_new_function_info_from_address"></a>

## Function `new_function_info_from_address`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(<br />    module_address: <b>address</b>,<br />    module_name: String,<br />    function_name: String,<br />): <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> &#123;<br />    <b>assert</b>!(<br />        <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&amp;module_name)),<br />        <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a><br />    );<br />    <b>assert</b>!(<br />        <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&amp;function_name)),<br />        <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a><br />    );<br />    <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> &#123;<br />        module_address,<br />        module_name,<br />        function_name,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility"></a>

## Function `check_dispatch_type_compatibility`

Check if the dispatch target function meets the type requirements of the disptach entry point.

framework_function is the dispatch native function defined in the aptos_framework.
dispatch_target is the function passed in by the user.

dispatch_target should have the same signature (same argument type, same generics constraint) except
that the framework_function will have a <code>&amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a></code> in the last argument that will instruct the VM which
function to jump to.

dispatch_target also needs to be public so the type signature will remain unchanged.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(framework_function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, dispatch_target: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(<br />    framework_function: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,<br />    dispatch_target: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,<br />): bool &#123;<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="function_info.md#0x1_function_info_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)<br />    );<br />    <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(dispatch_target);<br />    <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(framework_function, dispatch_target)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_function_info_load_module_from_function"></a>

## Function `load_module_from_function`

Load up a function into VM&apos;s loader and charge for its dependencies

It is &#42;&#42;critical&#42;&#42; to make sure that this function is invoked before <code>check_dispatch_type_compatibility</code>
or performing any other dispatching logic to ensure:
1. We properly charge gas for the function to dispatch.
2. The function is loaded in the cache so that we can perform further type checking/dispatching logic.

Calling <code>check_dispatch_type_compatibility_impl</code> or dispatch without loading up the module would yield an error
if such module isn&apos;t accessed previously in the transaction.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_load_module_from_function">load_module_from_function</a>(f: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_load_module_from_function">load_module_from_function</a>(f: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>) &#123;<br />    <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility_impl"></a>

## Function `check_dispatch_type_compatibility_impl`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, r: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>, r: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>): bool;<br /></code></pre>



</details>

<a id="0x1_function_info_is_identifier"></a>

## Function `is_identifier`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(s: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(s: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>



</details>

<a id="0x1_function_info_load_function_impl"></a>

## Function `load_function_impl`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>);<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_check_dispatch_type_compatibility_impl"></a>

### Function `check_dispatch_type_compatibility_impl`


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, r: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_1_load_function_impl"></a>

### Function `load_function_impl`


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &amp;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
