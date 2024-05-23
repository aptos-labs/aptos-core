
<a id="0x1_function_info"></a>

# Module `0x1::function_info`

The <code>function_info</code> module defines the <code>FunctionInfo</code> type which simulates a function pointer.


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


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::signer;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_function_info_FunctionInfo"></a>

## Struct `FunctionInfo`

A <code>String</code> holds a sequence of bytes which is guaranteed to be in utf8 format.


<pre><code>struct FunctionInfo has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>module_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>function_name: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_function_info_EINVALID_FUNCTION"></a>

Function specified in the FunctionInfo doesn&apos;t exist on chain.


<pre><code>const EINVALID_FUNCTION: u64 &#61; 2;<br/></code></pre>



<a id="0x1_function_info_EINVALID_IDENTIFIER"></a>

String is not a valid Move identifier


<pre><code>const EINVALID_IDENTIFIER: u64 &#61; 1;<br/></code></pre>



<a id="0x1_function_info_ENOT_ACTIVATED"></a>

Feature hasn&apos;t been activated yet.


<pre><code>const ENOT_ACTIVATED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_function_info_new_function_info"></a>

## Function `new_function_info`

Creates a new function info from names.


<pre><code>public fun new_function_info(module_signer: &amp;signer, module_name: string::String, function_name: string::String): function_info::FunctionInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_function_info(<br/>    module_signer: &amp;signer,<br/>    module_name: String,<br/>    function_name: String,<br/>): FunctionInfo &#123;<br/>    new_function_info_from_address(<br/>        signer::address_of(module_signer),<br/>        module_name,<br/>        function_name,<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_function_info_new_function_info_from_address"></a>

## Function `new_function_info_from_address`



<pre><code>public(friend) fun new_function_info_from_address(module_address: address, module_name: string::String, function_name: string::String): function_info::FunctionInfo<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun new_function_info_from_address(<br/>    module_address: address,<br/>    module_name: String,<br/>    function_name: String,<br/>): FunctionInfo &#123;<br/>    assert!(<br/>        is_identifier(string::bytes(&amp;module_name)),<br/>        EINVALID_IDENTIFIER<br/>    );<br/>    assert!(<br/>        is_identifier(string::bytes(&amp;function_name)),<br/>        EINVALID_IDENTIFIER<br/>    );<br/>    FunctionInfo &#123;<br/>        module_address,<br/>        module_name,<br/>        function_name,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility"></a>

## Function `check_dispatch_type_compatibility`

Check if the dispatch target function meets the type requirements of the disptach entry point.<br/><br/> framework_function is the dispatch native function defined in the aptos_framework.<br/> dispatch_target is the function passed in by the user.<br/><br/> dispatch_target should have the same signature (same argument type, same generics constraint) except<br/> that the framework_function will have a <code>&amp;FunctionInfo</code> in the last argument that will instruct the VM which<br/> function to jump to.<br/><br/> dispatch_target also needs to be public so the type signature will remain unchanged.


<pre><code>public(friend) fun check_dispatch_type_compatibility(framework_function: &amp;function_info::FunctionInfo, dispatch_target: &amp;function_info::FunctionInfo): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun check_dispatch_type_compatibility(<br/>    framework_function: &amp;FunctionInfo,<br/>    dispatch_target: &amp;FunctionInfo,<br/>): bool &#123;<br/>    assert!(<br/>        features::dispatchable_fungible_asset_enabled(),<br/>        error::aborted(ENOT_ACTIVATED)<br/>    );<br/>    load_function_impl(dispatch_target);<br/>    check_dispatch_type_compatibility_impl(framework_function, dispatch_target)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_function_info_load_module_from_function"></a>

## Function `load_module_from_function`

Load up a function into VM&apos;s loader and charge for its dependencies<br/><br/> It is &#42;&#42;critical&#42;&#42; to make sure that this function is invoked before <code>check_dispatch_type_compatibility</code><br/> or performing any other dispatching logic to ensure:<br/> 1. We properly charge gas for the function to dispatch.<br/> 2. The function is loaded in the cache so that we can perform further type checking/dispatching logic.<br/><br/> Calling <code>check_dispatch_type_compatibility_impl</code> or dispatch without loading up the module would yield an error<br/> if such module isn&apos;t accessed previously in the transaction.


<pre><code>public(friend) fun load_module_from_function(f: &amp;function_info::FunctionInfo)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun load_module_from_function(f: &amp;FunctionInfo) &#123;<br/>    load_function_impl(f)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_function_info_check_dispatch_type_compatibility_impl"></a>

## Function `check_dispatch_type_compatibility_impl`



<pre><code>fun check_dispatch_type_compatibility_impl(lhs: &amp;function_info::FunctionInfo, r: &amp;function_info::FunctionInfo): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun check_dispatch_type_compatibility_impl(lhs: &amp;FunctionInfo, r: &amp;FunctionInfo): bool;<br/></code></pre>



</details>

<a id="0x1_function_info_is_identifier"></a>

## Function `is_identifier`



<pre><code>fun is_identifier(s: &amp;vector&lt;u8&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun is_identifier(s: &amp;vector&lt;u8&gt;): bool;<br/></code></pre>



</details>

<a id="0x1_function_info_load_function_impl"></a>

## Function `load_function_impl`



<pre><code>fun load_function_impl(f: &amp;function_info::FunctionInfo)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun load_function_impl(f: &amp;FunctionInfo);<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_check_dispatch_type_compatibility_impl"></a>

### Function `check_dispatch_type_compatibility_impl`


<pre><code>fun check_dispatch_type_compatibility_impl(lhs: &amp;function_info::FunctionInfo, r: &amp;function_info::FunctionInfo): bool<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_load_function_impl"></a>

### Function `load_function_impl`


<pre><code>fun load_function_impl(f: &amp;function_info::FunctionInfo)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
