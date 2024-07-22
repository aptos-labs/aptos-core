
<a id="0x1_schedule_transaction_queue"></a>

# Module `0x1::schedule_transaction_queue`



-  [Struct `ScheduledTransaction`](#0x1_schedule_transaction_queue_ScheduledTransaction)
-  [Struct `TransactionId`](#0x1_schedule_transaction_queue_TransactionId)
-  [Resource `ScheduledQueue`](#0x1_schedule_transaction_queue_ScheduledQueue)
-  [Resource `ToRemove`](#0x1_schedule_transaction_queue_ToRemove)
-  [Function `new_transaction`](#0x1_schedule_transaction_queue_new_transaction)
-  [Function `initialize`](#0x1_schedule_transaction_queue_initialize)
-  [Function `insert`](#0x1_schedule_transaction_queue_insert)
-  [Function `cancel`](#0x1_schedule_transaction_queue_cancel)
-  [Function `get_ready_transactions`](#0x1_schedule_transaction_queue_get_ready_transactions)
-  [Function `finish_execution`](#0x1_schedule_transaction_queue_finish_execution)
-  [Function `reset`](#0x1_schedule_transaction_queue_reset)


<pre><code><b>use</b> <a href="aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;
<b>use</b> <a href="avl_tree.md#0x1_avl_queue">0x1::avl_queue</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table">0x1::iterable_table</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a id="0x1_schedule_transaction_queue_ScheduledTransaction"></a>

## Struct `ScheduledTransaction`



<pre><code><b>struct</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">ScheduledTransaction</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>scheduled_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_gas_unit: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>sender: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>payload: <a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_schedule_transaction_queue_TransactionId"></a>

## Struct `TransactionId`



<pre><code><b>struct</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_TransactionId">TransactionId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_schedule_transaction_queue_ScheduledQueue"></a>

## Resource `ScheduledQueue`



<pre><code><b>struct</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>queue: <a href="avl_tree.md#0x1_avl_queue_AVLqueue">avl_queue::AVLqueue</a>&lt;<a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_TransactionId">schedule_transaction_queue::TransactionId</a>, bool&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>items: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_TransactionId">schedule_transaction_queue::TransactionId</a>, <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">schedule_transaction_queue::ScheduledTransaction</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_schedule_transaction_queue_ToRemove"></a>

## Resource `ToRemove`



<pre><code><b>struct</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>num: <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_schedule_transaction_queue_new_transaction"></a>

## Function `new_transaction`



<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">new_transaction</a>(scheduled_time: u64, max_gas_unit: u64, payload: <a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>, sender: <b>address</b>): <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">schedule_transaction_queue::ScheduledTransaction</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_new_transaction">new_transaction</a>(scheduled_time: u64, max_gas_unit: u64, payload: EntryFunctionPayload, sender: <b>address</b>): <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">ScheduledTransaction</a> {
    // todo:: validate payload
    <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">ScheduledTransaction</a> {
        scheduled_time: scheduled_time,
        max_gas_unit,
        sender,
        payload,
    }
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>move_to</b>(framework, <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a> {
        queue: <a href="avl_tree.md#0x1_avl_queue_new">avl_queue::new</a>(<b>true</b>, 0, 0),
        items: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
    });
    <b>move_to</b>(framework, <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a> {
        num: <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_insert"></a>

## Function `insert`



<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">insert</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">schedule_transaction_queue::ScheduledTransaction</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_insert">insert</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn: <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">ScheduledTransaction</a>) <b>acquires</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender) == txn.sender, 1);
    <b>let</b> scheduled_queue = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a>&gt;(@aptos_framework);
    <b>let</b> id = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_TransactionId">TransactionId</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: sha3_256(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&txn)) };
    <b>if</b> (<a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_contains">table_with_length::contains</a>(&scheduled_queue.items, id)) {
        <b>return</b>
    };
    // <b>assert</b> <a href="timestamp.md#0x1_timestamp">timestamp</a> range
    <b>let</b> time = txn.scheduled_time;
    <b>if</b> (!<a href="avl_tree.md#0x1_avl_queue_has_key">avl_queue::has_key</a>(&scheduled_queue.queue, time)) {
        <a href="avl_tree.md#0x1_avl_queue_insert">avl_queue::insert</a>(&<b>mut</b> scheduled_queue.queue, time, <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_new">iterable_table::new</a>());
    };
    <b>let</b> (node_id, _) = <a href="avl_tree.md#0x1_avl_queue_search">avl_queue::search</a>(&scheduled_queue.queue, time);
    // Number of bits list node ID is shifted in an access key.
    // <b>const</b> SHIFT_ACCESS_LIST_NODE_ID: u8 = 33;
    <b>let</b> access_key = node_id &lt;&lt; 33;
    <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_add">iterable_table::add</a>(
        <a href="avl_tree.md#0x1_avl_queue_borrow_mut">avl_queue::borrow_mut</a>(&<b>mut</b> scheduled_queue.queue, access_key), id, <b>false</b>);
    <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> scheduled_queue.items, id, txn);
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_cancel"></a>

## Function `cancel`



<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_cancel">cancel</a>(sender: <b>address</b>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_cancel">cancel</a>(sender: <b>address</b>, txn_id: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a> {
    <b>let</b> scheduled_queue = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a>&gt;(@aptos_framework);
    <b>let</b> id = <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_TransactionId">TransactionId</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: txn_id };
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_contains">table_with_length::contains</a>(&scheduled_queue.items, id)) {
        <b>return</b>
    };
    <b>let</b> item = <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> scheduled_queue.items, id);
    <b>if</b> (item.sender != sender) {
        <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> scheduled_queue.items, id, item);
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_remove">iterable_table::remove</a>(<a href="avl_tree.md#0x1_avl_queue_borrow_mut">avl_queue::borrow_mut</a>(
            &<b>mut</b> scheduled_queue.queue, item.scheduled_time), id);
        <b>if</b> (<a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_length">iterable_table::length</a>(<a href="avl_tree.md#0x1_avl_queue_borrow">avl_queue::borrow</a>(&scheduled_queue.queue, item.scheduled_time)) == 0) {
            <b>let</b> empty_table = <a href="avl_tree.md#0x1_avl_queue_remove">avl_queue::remove</a>(&<b>mut</b> scheduled_queue.queue, item.scheduled_time);
            <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_destroy_empty">iterable_table::destroy_empty</a>(empty_table);
        }
    }
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_get_ready_transactions"></a>

## Function `get_ready_transactions`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_get_ready_transactions">get_ready_transactions</a>(<a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">schedule_transaction_queue::ScheduledTransaction</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_get_ready_transactions">get_ready_transactions</a>(<a href="timestamp.md#0x1_timestamp">timestamp</a>: u64, limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledTransaction">ScheduledTransaction</a>&gt; <b>acquires</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a>, <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a> {
    <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_reset">reset</a>();
    <b>let</b> scheduled_queue = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a>&gt;(@aptos_framework);
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&result) &lt; limit) {
        <b>let</b> head_key = <a href="avl_tree.md#0x1_avl_queue_get_head_key">avl_queue::get_head_key</a>(&scheduled_queue.queue);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&head_key)) {
            <b>return</b> result
        };
        <b>let</b> current_timestamp = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> head_key);
        <b>if</b> (current_timestamp &gt; <a href="timestamp.md#0x1_timestamp">timestamp</a>) {
            <b>return</b> result
        };
        <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <a href="avl_tree.md#0x1_avl_queue_pop_head">avl_queue::pop_head</a>(&<b>mut</b> scheduled_queue.queue);
        <b>let</b> key = <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_head_key">iterable_table::head_key</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>);
        <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&key)) {
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&result) &lt; limit) {
                <b>let</b> txn = <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&scheduled_queue.items, *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key));
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, *txn);
            };
            <b>let</b> (_, _, next) = <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_remove_iter">iterable_table::remove_iter</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key));
            key = next;
        };
        <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_destroy_empty">iterable_table::destroy_empty</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>);
    };
    result
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_finish_execution"></a>

## Function `finish_execution`

Increment at every scheduled transaction without affect parallelism


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_finish_execution">finish_execution</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_finish_execution">finish_execution</a>() <b>acquires</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a> {
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a>&gt;(@aptos_framework);
    <a href="aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&<b>mut</b> to_remove.num, 1);
}
</code></pre>



</details>

<a id="0x1_schedule_transaction_queue_reset"></a>

## Function `reset`

Reset at beginning of each block


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_reset">reset</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_reset">reset</a>() <b>acquires</b> <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a>, <a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a> {
    <b>let</b> to_remove = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ToRemove">ToRemove</a>&gt;(@aptos_framework);
    <b>let</b> num_to_remove = <a href="aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&to_remove.num);
    <a href="aggregator_v2.md#0x1_aggregator_v2_sub">aggregator_v2::sub</a>(&<b>mut</b> to_remove.num, num_to_remove);
    <b>let</b> scheduled_queue = <b>borrow_global_mut</b>&lt;<a href="schedule_transaction_queue.md#0x1_schedule_transaction_queue_ScheduledQueue">ScheduledQueue</a>&gt;(@aptos_framework);
    <b>while</b> (num_to_remove &gt; 0) {
        <b>let</b> head_key = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> <a href="avl_tree.md#0x1_avl_queue_get_head_key">avl_queue::get_head_key</a>(&scheduled_queue.queue));
        <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <a href="avl_tree.md#0x1_avl_queue_pop_head">avl_queue::pop_head</a>(&<b>mut</b> scheduled_queue.queue);
        <b>let</b> key = <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_head_key">iterable_table::head_key</a>(&<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>);
        <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&key) && num_to_remove &gt; 0) {
            <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> scheduled_queue.items, *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key));
            <b>let</b> (_, _, next) = <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_remove_iter">iterable_table::remove_iter</a>(&<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key));
            key = next;
            num_to_remove = num_to_remove - 1;
        };
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&key)) {
            <a href="../../aptos-stdlib/doc/iterable_table.md#0x1_iterable_table_destroy_empty">iterable_table::destroy_empty</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>);
        } <b>else</b> {
            <a href="avl_tree.md#0x1_avl_queue_insert">avl_queue::insert</a>(&<b>mut</b> scheduled_queue.queue, head_key, <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>);
            <b>return</b>
        }
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
