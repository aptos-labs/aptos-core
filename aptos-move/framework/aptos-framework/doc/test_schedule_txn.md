
<a id="0x1_test_schedule_txn"></a>

# Module `0x1::test_schedule_txn`



-  [Resource `TestStruct`](#0x1_test_schedule_txn_TestStruct)
-  [Function `foo`](#0x1_test_schedule_txn_foo)
-  [Function `foo_with_arg`](#0x1_test_schedule_txn_foo_with_arg)
-  [Function `foo_with_signer_and_arg`](#0x1_test_schedule_txn_foo_with_signer_and_arg)
-  [Function `foo_with_new_storage`](#0x1_test_schedule_txn_foo_with_new_storage)
-  [Function `cancel`](#0x1_test_schedule_txn_cancel)
-  [Function `recurring`](#0x1_test_schedule_txn_recurring)
-  [Function `gen_payload`](#0x1_test_schedule_txn_gen_payload)


<pre><code><b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue">0x1::schedule_transaction_queue</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a id="0x1_test_schedule_txn_TestStruct"></a>

## Resource `TestStruct`



<pre><code><b>struct</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>b: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_test_schedule_txn_foo"></a>

## Function `foo`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo">foo</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo">foo</a>() <b>acquires</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a> {
    <b>let</b> v = <b>borrow_global_mut</b>&lt;<a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a>&gt;(@core_resources);
    v.a = 0;
    v.b = 0;
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_foo_with_arg"></a>

## Function `foo_with_arg`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_arg">foo_with_arg</a>(a: u64, b: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_arg">foo_with_arg</a>(a: u64, b: u64) <b>acquires</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a> {
    <b>let</b> v = <b>borrow_global_mut</b>&lt;<a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a>&gt;(@core_resources);
    v.a = a;
    v.b = b;
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_foo_with_signer_and_arg"></a>

## Function `foo_with_signer_and_arg`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_signer_and_arg">foo_with_signer_and_arg</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, value: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_signer_and_arg">foo_with_signer_and_arg</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, value: u64) {
    <a href="aptos_account.md#0x1_aptos_account_transfer">aptos_account::transfer</a>(sender, @0x12345, value);
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_foo_with_new_storage"></a>

## Function `foo_with_new_storage`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_new_storage">foo_with_new_storage</a>(_val: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_foo_with_new_storage">foo_with_new_storage</a>(_val: u64) {
    <a href="object.md#0x1_object_create_object">object::create_object</a>(@core_resources);
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_cancel"></a>

## Function `cancel`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_cancel">cancel</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_cancel">schedule_transaction_queue::cancel</a>(sender, txn_id);
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_recurring"></a>

## Function `recurring`



<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_recurring">recurring</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_recurring">recurring</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>if</b> (!<b>exists</b>&lt;<a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a>&gt;(addr)) {
        <b>move_to</b>(sender, <a href="test_schedule_txn.md#0x1_test_schedule_txn_TestStruct">TestStruct</a> { a: 1, b: 2 });
    };
    <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
        <a href="timestamp.md#0x1_timestamp">timestamp</a> + 1,
        100000,
        <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"recurring", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]),
        addr,
    );
    <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> % 3 == 0) {
        <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
            <a href="timestamp.md#0x1_timestamp">timestamp</a> + 2,
            1000,
            <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"foo", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]),
            addr,
        );
        <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
    };
    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> % 3 == 1) {
        <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
            <a href="timestamp.md#0x1_timestamp">timestamp</a> + 3,
            1000,
            <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"foo_with_arg", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="timestamp.md#0x1_timestamp">timestamp</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&(<a href="timestamp.md#0x1_timestamp">timestamp</a> % 1234))]),
            addr,
        );
        <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
    };
    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> % 3 == 2) {
        <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
            <a href="timestamp.md#0x1_timestamp">timestamp</a> + 4,
            1000,
            <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"foo_with_signer_and_arg", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&100)]),
            addr,
        );
        <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
    };

    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> % 5 == 0) {
        <b>let</b> i = 50;
        <b>while</b> (i &gt; 0) {
            <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
                <a href="timestamp.md#0x1_timestamp">timestamp</a> + 5 + i % 2,
                1000,
                <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"foo_with_new_storage", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&i)]),
                addr,
            );
            <b>let</b> id = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
            <b>if</b> (i % 3 == 0) {
                // schedule a closer txn <b>to</b> test cancel
                <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
                    <a href="timestamp.md#0x1_timestamp">timestamp</a> + i,
                    1000,
                    <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"cancel", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&id)]),
                    addr,
                );
                <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
            };
            i = i - 1;
        }
    };

    <b>if</b> (<a href="timestamp.md#0x1_timestamp">timestamp</a> % 5 == 1) {
        // out of gas
        <b>let</b> txn = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">schedule_transaction_queue::new_transaction</a>(
            <a href="timestamp.md#0x1_timestamp">timestamp</a> + 4,
            0,
            <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(b"foo_with_signer_and_arg", <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&100)]),
            addr,
        );
        <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">schedule_transaction_queue::insert</a>(sender, txn);
    }
}
</code></pre>



</details>

<a id="0x1_test_schedule_txn_gen_payload"></a>

## Function `gen_payload`



<pre><code><b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, args: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="test_schedule_txn.md#0x1_test_schedule_txn_gen_payload">gen_payload</a>(name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, args: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): EntryFunctionPayload {
    <a href="transaction_context.md#0x1_transaction_context_new_entry_function_payload">transaction_context::new_entry_function_payload</a>(@0x1, <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="test_schedule_txn.md#0x1_test_schedule_txn">test_schedule_txn</a>"), <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(name), <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], args)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
