
<a name="0x1_promise"></a>

# Module `0x1::promise`



-  [Struct `Promise`](#0x1_promise_Promise)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_promise_new)
-  [Function `get_value`](#0x1_promise_get_value)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_promise_Promise"></a>

## Struct `Promise`



<pre><code><b>struct</b> <a href="promise.md#0x1_promise_Promise">Promise</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_promise_EPROMISE_NOT_RESOLVED"></a>

The error code raised when <code>get_value</code> function is called before
resolving the promise.


<pre><code><b>const</b> <a href="promise.md#0x1_promise_EPROMISE_NOT_RESOLVED">EPROMISE_NOT_RESOLVED</a>: u64 = 1;
</code></pre>



<a name="0x1_promise_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="promise.md#0x1_promise_new">new</a>(id: <b>address</b>): <a href="promise.md#0x1_promise_Promise">promise::Promise</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="promise.md#0x1_promise_new">new</a>(id: <b>address</b>): <a href="promise.md#0x1_promise_Promise">Promise</a> {
    <a href="promise.md#0x1_promise_Promise">Promise</a> {
        value: 0,
        id: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(id)
    }
}
</code></pre>



</details>

<a name="0x1_promise_get_value"></a>

## Function `get_value`



<pre><code><b>public</b> <b>fun</b> <a href="promise.md#0x1_promise_get_value">get_value</a>(<a href="promise.md#0x1_promise">promise</a>: &<a href="promise.md#0x1_promise_Promise">promise::Promise</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="promise.md#0x1_promise_get_value">get_value</a>(<a href="promise.md#0x1_promise">promise</a>: &<a href="promise.md#0x1_promise_Promise">Promise</a>): u128 {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&<a href="promise.md#0x1_promise">promise</a>.id), <a href="promise.md#0x1_promise_EPROMISE_NOT_RESOLVED">EPROMISE_NOT_RESOLVED</a>);
    <a href="promise.md#0x1_promise">promise</a>.value
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
