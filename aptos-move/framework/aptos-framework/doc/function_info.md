
<a id="0x1_function_info"></a>

# Module `0x1::function_info`

The <code><a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">string</a></code> module defines the <code>String</code> type which represents UTF8 encoded strings.


-  [Struct `FunctionInfo`](#0x1_function_info_FunctionInfo)
-  [Constants](#@Constants_0)
-  [Function `new_function_info`](#0x1_function_info_new_function_info)
-  [Function `check_dispatch_type_compatibility`](#0x1_function_info_check_dispatch_type_compatibility)
-  [Function `check_dispatch_type_compatibility_impl`](#0x1_function_info_check_dispatch_type_compatibility_impl)
-  [Function `is_identifier`](#0x1_function_info_is_identifier)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
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



<a id="0x1_function_info_new_function_info"></a>

## Function `new_function_info`

Creates a new function info from names.


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="function_info.md#0x1_function_info_new_function_info">new_function_info</a>(
    module_address: <b>address</b>,
    module_name: String,
    function_name: String,
): <a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a> {
    <b>assert</b>!(<a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&module_name)), <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>);
    <b>assert</b>!(<a href="function_info.md#0x1_function_info_is_identifier">is_identifier</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&function_name)), <a href="function_info.md#0x1_function_info_EINVALID_IDENTIFIER">EINVALID_IDENTIFIER</a>);
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



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(lhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, rhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">check_dispatch_type_compatibility</a>(
    lhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,
    rhs: &<a href="function_info.md#0x1_function_info_FunctionInfo">FunctionInfo</a>,
): bool {
    <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility_impl">check_dispatch_type_compatibility_impl</a>(lhs, rhs)
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


[move-book]: https://aptos.dev/move/book/SUMMARY
