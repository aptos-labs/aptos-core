
<a id="0x1_reflect"></a>

# Module `0x1::reflect`

Functionality for reflection in Move.


-  [Enum `ReflectionError`](#0x1_reflect_ReflectionError)
-  [Constants](#@Constants_0)
-  [Function `resolve`](#0x1_reflect_resolve)
-  [Function `error_code`](#0x1_reflect_error_code)
-  [Function `native_resolve`](#0x1_reflect_native_resolve)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="features.md#0x1_features">0x1::features</a>;
<b>use</b> 0x1::result;
<b>use</b> <a href="string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_reflect_ReflectionError"></a>

## Enum `ReflectionError`

Represents errors returned by the reflection API.
TODO: make this public once language version 2.4 is available


<pre><code>enum <a href="reflect.md#0x1_reflect_ReflectionError">ReflectionError</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>InvalidIdentifier</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FunctionNotFound</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FunctionNotAccessible</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FunctionIncompatibleType</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>FunctionNotInstantiated</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_reflect_E_FEATURE_NOT_ENABLED"></a>

This error indicates that the reflection feature is not enabled.


<pre><code><b>const</b> <a href="reflect.md#0x1_reflect_E_FEATURE_NOT_ENABLED">E_FEATURE_NOT_ENABLED</a>: u64 = 0;
</code></pre>



<a id="0x1_reflect_resolve"></a>

## Function `resolve`

Resolves a function specified by address and symbolic name, with expected type, into a typed function value.

Example usage:

```
let fn : |address|u64 has store = reflect::resolve(@somewhere, utf8(b"mod"), utf8(b"fn")).unwrap();
assert!(fn(my_addr) == some_value)
```

See <code><a href="reflect.md#0x1_reflect_ReflectionError">ReflectionError</a></code> for the possible errors which can result. On successful resolution,
a function value is returned which can be safely used in future executions as indicated by the requested
type.

In order to be accessible, the resolved function must be public. This prevents reflection to
work around the languages modular encapsulation guarantees.

The resolved function can be generic, in which case the instantiation must be inferrible
from the provided <code>FuncType</code>. For example, <code><b>public</b> <b>fun</b> foo&lt;T&gt;(T)</code>, with <code>FunType = |u64|</code>,
<code>T = u64</code> can be derived. If not all type parameters can be inferred, an error will be
produced.


<pre><code><b>public</b> <b>fun</b> <a href="reflect.md#0x1_reflect_resolve">resolve</a>&lt;FuncType&gt;(addr: <b>address</b>, module_name: &<a href="string.md#0x1_string_String">string::String</a>, func_name: &<a href="string.md#0x1_string_String">string::String</a>): <a href="result.md#0x1_result_Result">result::Result</a>&lt;FuncType, <a href="reflect.md#0x1_reflect_ReflectionError">reflect::ReflectionError</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reflect.md#0x1_reflect_resolve">resolve</a>&lt;FuncType&gt;(
    addr: <b>address</b>, module_name: &String, func_name: &String
): Result&lt;FuncType, <a href="reflect.md#0x1_reflect_ReflectionError">ReflectionError</a>&gt; {
    <b>assert</b>!(
        <a href="features.md#0x1_features_is_function_reflection_enabled">features::is_function_reflection_enabled</a>(),
        <a href="error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reflect.md#0x1_reflect_E_FEATURE_NOT_ENABLED">E_FEATURE_NOT_ENABLED</a>)
    );
    <a href="reflect.md#0x1_reflect_native_resolve">native_resolve</a>(addr, module_name, func_name)
}
</code></pre>



</details>

<a id="0x1_reflect_error_code"></a>

## Function `error_code`

Returns numerical code associated with error.


<pre><code><b>public</b> <b>fun</b> <a href="reflect.md#0x1_reflect_error_code">error_code</a>(self: <a href="reflect.md#0x1_reflect_ReflectionError">reflect::ReflectionError</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reflect.md#0x1_reflect_error_code">error_code</a>(self: <a href="reflect.md#0x1_reflect_ReflectionError">ReflectionError</a>): u64 {
    match(self) {
        InvalidIdentifier =&gt; 0,
        FunctionNotFound =&gt; 1,
        FunctionNotAccessible =&gt; 2,
        FunctionIncompatibleType =&gt; 3,
        FunctionNotInstantiated =&gt; 4
    }
}
</code></pre>



</details>

<a id="0x1_reflect_native_resolve"></a>

## Function `native_resolve`



<pre><code><b>fun</b> <a href="reflect.md#0x1_reflect_native_resolve">native_resolve</a>&lt;FuncType&gt;(addr: <b>address</b>, module_name: &<a href="string.md#0x1_string_String">string::String</a>, func_name: &<a href="string.md#0x1_string_String">string::String</a>): <a href="result.md#0x1_result_Result">result::Result</a>&lt;FuncType, <a href="reflect.md#0x1_reflect_ReflectionError">reflect::ReflectionError</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="reflect.md#0x1_reflect_native_resolve">native_resolve</a>&lt;FuncType&gt;(
    addr: <b>address</b>, module_name: &String, func_name: &String
): Result&lt;FuncType, <a href="reflect.md#0x1_reflect_ReflectionError">ReflectionError</a>&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
