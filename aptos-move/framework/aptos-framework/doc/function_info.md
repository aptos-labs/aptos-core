
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
    -  [Function `new_function_info`](#@Specification_1_new_function_info)
    -  [Function `new_function_info_from_address`](#@Specification_1_new_function_info_from_address)
    -  [Function `check_dispatch_type_compatibility`](#@Specification_1_check_dispatch_type_compatibility)
    -  [Function `load_module_from_function`](#@Specification_1_load_module_from_function)
    -  [Function `check_dispatch_type_compatibility_impl`](#@Specification_1_check_dispatch_type_compatibility_impl)
    -  [Function `is_identifier`](#@Specification_1_is_identifier)
    -  [Function `load_function_impl`](#@Specification_1_load_function_impl)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_function_info_FunctionInfo"></a>

## Struct `FunctionInfo`

A <code>String</code> holds a sequence of bytes which is guaranteed to be in utf8 format.


<pre><code><b>struct</b> <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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

Function specified in the FunctionInfo doesn't exist on chain.


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_EINVALID_FUNCTION">EINVALID_FUNCTION</a>: u64 = 2;
</code></pre>



<a id="0x1_function_info_EINVALID_IDENTIFIER"></a>

String is not a valid Move identifier


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>: u64 = 1;
</code></pre>



<a id="0x1_function_info_ENOT_ACTIVATED"></a>

Feature hasn't been activated yet.


<pre><code><b>const</b> <a href="function_info.md#0x1_function_info_ENOT_ACTIVATED">ENOT_ACTIVATED</a>: u64 = 3;
</code></pre>



<a id="0x1_function_info_new_function_info"></a>

## Function `new_function_info`

Creates a new function info from names.


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(module_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(
    module_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    module_name: String,
    function_name: String,
): <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
    <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(module_signer),
        module_name,
        function_name,
    )
}
</code></pre>



</details>

<a id="0x1_function_info_new_function_info_from_address"></a>

## Function `new_function_info_from_address`



<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(
    module_address: <b>address</b>,
    module_name: String,
    function_name: String,
): <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
    <b>assert</b>!(
        <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&module_name)),
        <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>
    );
    <b>assert</b>!(
        <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&function_name)),
        <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>
    );
    <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
        module_address,
        module_name,
        function_name,
    }
}
</code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility"></a>

## Function `check_dispatch_type_compatibility`

Check if the dispatch target function meets the type requirements of the disptach entry point.

framework_function is the dispatch native function defined in the aptos_framework.
dispatch_target is the function passed in by the user.

dispatch_target should have the same signature (same argument type, same generics constraint) except
that the framework_function will have a <code>&<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a></code> in the last argument that will instruct the VM which
function to jump to.

dispatch_target also needs to be public so the type signature will remain unchanged.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(framework_function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, dispatch_target: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(
    framework_function: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,
    dispatch_target: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,
): bool {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_dispatchable_fungible_asset_enabled">features::dispatchable_fungible_asset_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_aborted">error::aborted</a>(<a href="function_info.md#0x1_function_info_ENOT_ACTIVATED">ENOT_ACTIVATED</a>)
    );
    <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(dispatch_target);
    <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(framework_function, dispatch_target)
}
</code></pre>



</details>

<a id="0x1_function_info_load_module_from_function"></a>

## Function `load_module_from_function`

Load up a function into VM's loader and charge for its dependencies

It is **critical** to make sure that this function is invoked before <code>check_dispatch_type_compatibility</code>
or performing any other dispatching logic to ensure:
1. We properly charge gas for the function to dispatch.
2. The function is loaded in the cache so that we can perform further type checking/dispatching logic.

Calling <code>check_dispatch_type_compatibility_impl</code> or dispatch without loading up the module would yield an error
if such module isn't accessed previously in the transaction.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_load_module_from_function">load_module_from_function</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_load_module_from_function">load_module_from_function</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>) {
    <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f)
}
</code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility_impl"></a>

## Function `check_dispatch_type_compatibility_impl`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, r: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>, r: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>): bool;
</code></pre>



</details>

<a id="0x1_function_info_is_identifier"></a>

## Function `is_identifier`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="0x1_function_info_load_function_impl"></a>

## Function `load_function_impl`



<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>);
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="0x1_function_info_spec_is_identifier"></a>


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(s: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



<a id="@Specification_1_new_function_info"></a>

### Function `new_function_info`


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(module_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>
</code></pre>




<pre><code><b>aborts_if</b> !<a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(module_name));
<b>aborts_if</b> !<a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(function_name));
<b>ensures</b> result == <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
    module_address: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(module_signer),
    module_name,
    function_name,
};
</code></pre>



<a id="@Specification_1_new_function_info_from_address"></a>

### Function `new_function_info_from_address`


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info_from_address">new_function_info_from_address</a>(module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>
</code></pre>




<pre><code><b>aborts_if</b> !<a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(module_name));
<b>aborts_if</b> !<a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(function_name));
<b>ensures</b> result == <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
    module_address,
    module_name,
    function_name,
};
</code></pre>



<a id="@Specification_1_check_dispatch_type_compatibility"></a>

### Function `check_dispatch_type_compatibility`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(framework_function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, dispatch_target: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_load_module_from_function"></a>

### Function `load_module_from_function`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_load_module_from_function">load_module_from_function</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_check_dispatch_type_compatibility_impl"></a>

### Function `check_dispatch_type_compatibility_impl`


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, r: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_is_identifier"></a>

### Function `is_identifier`


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="function_info.md#0x1_function_info_spec_is_identifier">spec_is_identifier</a>(s);
</code></pre>



<a id="@Specification_1_load_function_impl"></a>

### Function `load_function_impl`


<pre><code><b>fun</b> <a href="function_info.md#0x1_function_info_load_function_impl">load_function_impl</a>(f: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
