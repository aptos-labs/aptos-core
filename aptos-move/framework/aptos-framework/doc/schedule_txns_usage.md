
<a id="0x1_schedule_txns_usage"></a>

# Module `0x1::schedule_txns_usage`



-  [Struct `State`](#0x1_schedule_txns_usage_State)
-  [Function `step`](#0x1_schedule_txns_usage_step)
-  [Function `test_initialize`](#0x1_schedule_txns_usage_test_initialize)
-  [Function `test_insert_transactions`](#0x1_schedule_txns_usage_test_insert_transactions)
-  [Function `test_cancel_transaction`](#0x1_schedule_txns_usage_test_cancel_transaction)
-  [Function `test_shutdown`](#0x1_schedule_txns_usage_test_shutdown)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="scheduled_txns.md#0x1_scheduled_txns">0x1::scheduled_txns</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_schedule_txns_usage_State"></a>

## Struct `State`



<pre><code><b>struct</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_State">State</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_schedule_txns_usage_step"></a>

## Function `step`



<pre><code>#[persistent]
<b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_step">step</a>(state: <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_State">schedule_txns_usage::State</a>, _s: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_step">step</a>(state: <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_State">State</a>, _s: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;) {
    <b>if</b> (state.value &lt; 10) {
        state.value = state.value + 1;
    }
}
</code></pre>



</details>

<a id="0x1_schedule_txns_usage_test_initialize"></a>

## Function `test_initialize`



<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_initialize">test_initialize</a>(aptos: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_initialize">test_initialize</a>(aptos: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="scheduled_txns.md#0x1_scheduled_txns_initialize">scheduled_txns::initialize</a>(aptos);
}
</code></pre>



</details>

<a id="0x1_schedule_txns_usage_test_insert_transactions"></a>

## Function `test_insert_transactions`



<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_insert_transactions">test_insert_transactions</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, current_time_ms: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_insert_transactions">test_insert_transactions</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, current_time_ms: u64) {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"test_insert_transactions"));
    <b>let</b> state = <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_State">State</a> { value: 8 };
    <b>let</b> foo = |s: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>&gt;| <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_step">step</a>(state, s);

    <b>let</b> txn1 = <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">scheduled_txns::new_scheduled_transaction</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        current_time_ms + 100000,
        0,
        20,
        <b>false</b>,
        foo
    );
    <b>let</b> txn2 = <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">scheduled_txns::new_scheduled_transaction</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        current_time_ms + 200000,
        0,
        20,
        <b>false</b>,
        foo
    );
    <b>let</b> txn3 = <a href="scheduled_txns.md#0x1_scheduled_txns_new_scheduled_transaction">scheduled_txns::new_scheduled_transaction</a>(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user),
        current_time_ms + 300000,
        0,
        20,
        <b>false</b>,
        foo
    );

    <a href="scheduled_txns.md#0x1_scheduled_txns_insert">scheduled_txns::insert</a>(user, txn1);
    <a href="scheduled_txns.md#0x1_scheduled_txns_insert">scheduled_txns::insert</a>(user, txn2);
    <a href="scheduled_txns.md#0x1_scheduled_txns_insert">scheduled_txns::insert</a>(user, txn3);

    //<b>assert</b>!(3 == <a href="scheduled_txns.md#0x1_scheduled_txns_get_num_txns">scheduled_txns::get_num_txns</a>(), <a href="scheduled_txns.md#0x1_scheduled_txns_get_num_txns">scheduled_txns::get_num_txns</a>());
}
</code></pre>



</details>

<a id="0x1_schedule_txns_usage_test_cancel_transaction"></a>

## Function `test_cancel_transaction`



<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_cancel_transaction">test_cancel_transaction</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_cancel_transaction">test_cancel_transaction</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    //<a href="scheduled_txns.md#0x1_scheduled_txns_cancel">scheduled_txns::cancel</a>(user, key);
}
</code></pre>



</details>

<a id="0x1_schedule_txns_usage_test_shutdown"></a>

## Function `test_shutdown`



<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_shutdown">test_shutdown</a>(aptos: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="schedule_txns_usage.md#0x1_schedule_txns_usage_test_shutdown">test_shutdown</a>(aptos: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="scheduled_txns.md#0x1_scheduled_txns_shutdown">scheduled_txns::shutdown</a>(aptos);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
