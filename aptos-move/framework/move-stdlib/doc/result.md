
<a id="0x1_result"></a>

# Module `0x1::result`



-  [Enum `Result`](#0x1_result_Result)
-  [Constants](#@Constants_0)
-  [Function `is_ok`](#0x1_result_is_ok)
-  [Function `is_err`](#0x1_result_is_err)
-  [Function `unwrap`](#0x1_result_unwrap)
-  [Function `unwrap_err`](#0x1_result_unwrap_err)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
</code></pre>



<a id="0x1_result_Result"></a>

## Enum `Result`

Represents the result of some computation, either a value <code>T</code> or an error <code>E</code>.


<pre><code>enum <a href="result.md#0x1_result_Result">Result</a>&lt;T, E&gt; <b>has</b> <b>copy</b>, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Ok</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Err</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>0: E</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_result_EUNWRAP_ERR"></a>

Attempt to unwrap error but found value


<pre><code><b>const</b> <a href="result.md#0x1_result_EUNWRAP_ERR">EUNWRAP_ERR</a>: u64 = 0;
</code></pre>



<a id="0x1_result_EUNWRAP_OK"></a>

Attempt to unwrap value but found error


<pre><code><b>const</b> <a href="result.md#0x1_result_EUNWRAP_OK">EUNWRAP_OK</a>: u64 = 0;
</code></pre>



<a id="0x1_result_is_ok"></a>

## Function `is_ok`

Checks whether the result is Ok.


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_is_ok">is_ok</a>&lt;T, E&gt;(self: &<a href="result.md#0x1_result_Result">result::Result</a>&lt;T, E&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_is_ok">is_ok</a>&lt;T, E&gt;(self: &<a href="result.md#0x1_result_Result">Result</a>&lt;T, E&gt;): bool {
    self is Ok
}
</code></pre>



</details>

<a id="0x1_result_is_err"></a>

## Function `is_err`

Checks whether the result is Err.


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_is_err">is_err</a>&lt;T, E&gt;(self: &<a href="result.md#0x1_result_Result">result::Result</a>&lt;T, E&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_is_err">is_err</a>&lt;T, E&gt;(self: &<a href="result.md#0x1_result_Result">Result</a>&lt;T, E&gt;): bool {
    self is Err
}
</code></pre>



</details>

<a id="0x1_result_unwrap"></a>

## Function `unwrap`

Unpacks the <code>T</code> of Ok or aborts.


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_unwrap">unwrap</a>&lt;T, E&gt;(self: <a href="result.md#0x1_result_Result">result::Result</a>&lt;T, E&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_unwrap">unwrap</a>&lt;T, E&gt;(self: <a href="result.md#0x1_result_Result">Result</a>&lt;T, E&gt;): T {
    match (self) {
        Ok(x) =&gt; x,
        _ =&gt; <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="result.md#0x1_result_EUNWRAP_OK">EUNWRAP_OK</a>)
    }
}
</code></pre>



</details>

<a id="0x1_result_unwrap_err"></a>

## Function `unwrap_err`

Unpacks the <code>E</code> of Err or aborts.


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_unwrap_err">unwrap_err</a>&lt;T, E&gt;(self: <a href="result.md#0x1_result_Result">result::Result</a>&lt;T, E&gt;): E
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="result.md#0x1_result_unwrap_err">unwrap_err</a>&lt;T, E&gt;(self: <a href="result.md#0x1_result_Result">Result</a>&lt;T, E&gt;): E {
    match (self) {
        Err(x) =&gt; x,
        _ =&gt; <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="result.md#0x1_result_EUNWRAP_ERR">EUNWRAP_ERR</a>)
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
