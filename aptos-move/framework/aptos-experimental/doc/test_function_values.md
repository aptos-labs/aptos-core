
<a id="0x7_test_function_values"></a>

# Module `0x7::test_function_values`



-  [Function `transfer_and_create_account`](#0x7_test_function_values_transfer_and_create_account)


<pre><code></code></pre>



<a id="0x7_test_function_values_transfer_and_create_account"></a>

## Function `transfer_and_create_account`



<pre><code><b>fun</b> <a href="test_function_values.md#0x7_test_function_values_transfer_and_create_account">transfer_and_create_account</a>(some_f: |u64|u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="test_function_values.md#0x7_test_function_values_transfer_and_create_account">transfer_and_create_account</a>(some_f: |u64|u64): u64 {
    some_f(3)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
