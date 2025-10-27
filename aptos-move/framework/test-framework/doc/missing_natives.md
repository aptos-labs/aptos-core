
<a id="0x1_test_missing_native"></a>

# Module `0x1::test_missing_native`

Test framework module for testing missing native function error handling.
This module should never be included in production builds.


-  [Function `missing_native`](#0x1_test_missing_native_missing_native)
-  [Function `public_missing_native`](#0x1_test_missing_native_public_missing_native)
-  [Function `missing_native_function`](#0x1_test_missing_native_missing_native_function)


<pre><code></code></pre>



<a id="0x1_test_missing_native_missing_native"></a>

## Function `missing_native`

Native function declaration without implementation - FOR TESTING ONLY


<pre><code><b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_missing_native">missing_native</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_missing_native">missing_native</a>();
</code></pre>



</details>

<a id="0x1_test_missing_native_public_missing_native"></a>

## Function `public_missing_native`

Public native function declaration without implementation - FOR TESTING ONLY


<pre><code><b>public</b> <b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_public_missing_native">public_missing_native</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_public_missing_native">public_missing_native</a>();
</code></pre>



</details>

<a id="0x1_test_missing_native_missing_native_function"></a>

## Function `missing_native_function`

Public wrapper function that calls the missing native
This function is used to trigger the missing native function error during tests.


<pre><code><b>public</b> <b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_missing_native_function">missing_native_function</a>(framework: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="missing_natives.md#0x1_test_missing_native_missing_native_function">missing_native_function</a>(framework: &signer) {
    <a href="missing_natives.md#0x1_test_missing_native_missing_native">missing_native</a>();
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
