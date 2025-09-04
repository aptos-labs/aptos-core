
<a id="0x7_pre_cancellation_tracker"></a>

# Module `0x7::pre_cancellation_tracker`

Without this feature, today, the MM will need to wait for an order
to be placed and confirmed on the orderbook, so that the MM can
get the order id to be able to cancel the order - this means minimum time
required to be able to submit a transaction to cancel the order is
end-to-end blockchain latency (~500 ms). This adds support for an MM to pre-cancel an order,
which means specify that this order with a client order id is cancelled even before the order is placed.
This reduces the latency to submit a cancellation transaction from 500 ms to 0.


-  [Struct `PreCancellationTracker`](#0x7_pre_cancellation_tracker_PreCancellationTracker)
-  [Struct `ExpirationAndOrderId`](#0x7_pre_cancellation_tracker_ExpirationAndOrderId)
-  [Constants](#@Constants_0)
-  [Function `new_pre_cancellation_tracker`](#0x7_pre_cancellation_tracker_new_pre_cancellation_tracker)
-  [Function `pre_cancel_order_for_tracker`](#0x7_pre_cancellation_tracker_pre_cancel_order_for_tracker)
-  [Function `is_pre_cancelled`](#0x7_pre_cancellation_tracker_is_pre_cancelled)
-  [Function `garbage_collect`](#0x7_pre_cancellation_tracker_garbage_collect)


<pre><code><b>use</b> <a href="../../velor-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../velor-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="order_book_types.md#0x7_order_book_types">0x7::order_book_types</a>;
</code></pre>



<a id="0x7_pre_cancellation_tracker_PreCancellationTracker"></a>

## Struct `PreCancellationTracker`



<pre><code><b>struct</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pre_cancellation_window_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_with_order_ids: <a href="../../velor-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_ExpirationAndOrderId">pre_cancellation_tracker::ExpirationAndOrderId</a>, bool&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>account_order_ids: <a href="../../velor-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_pre_cancellation_tracker_ExpirationAndOrderId"></a>

## Struct `ExpirationAndOrderId`



<pre><code><b>struct</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_ExpirationAndOrderId">ExpirationAndOrderId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>expiration_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>account_order_id: <a href="order_book_types.md#0x7_order_book_types_AccountClientOrderId">order_book_types::AccountClientOrderId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_pre_cancellation_tracker_DUPLICATE_ORDER_PLACEMENT"></a>



<pre><code><b>const</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_DUPLICATE_ORDER_PLACEMENT">DUPLICATE_ORDER_PLACEMENT</a>: u64 = 1;
</code></pre>



<a id="0x7_pre_cancellation_tracker_MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL"></a>



<pre><code><b>const</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL">MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL</a>: u64 = 10;
</code></pre>



<a id="0x7_pre_cancellation_tracker_new_pre_cancellation_tracker"></a>

## Function `new_pre_cancellation_tracker`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_new_pre_cancellation_tracker">new_pre_cancellation_tracker</a>(expiration_time_secs: u64): <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_new_pre_cancellation_tracker">new_pre_cancellation_tracker</a>(expiration_time_secs: u64): <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a> {
    <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a> {
        pre_cancellation_window_secs: expiration_time_secs,
        expiration_with_order_ids: <a href="../../velor-framework/doc/big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>(),
        account_order_ids: <a href="../../velor-framework/doc/big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>()
    }
}
</code></pre>



</details>

<a id="0x7_pre_cancellation_tracker_pre_cancel_order_for_tracker"></a>

## Function `pre_cancel_order_for_tracker`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_pre_cancel_order_for_tracker">pre_cancel_order_for_tracker</a>(tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>, <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, client_order_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_pre_cancel_order_for_tracker">pre_cancel_order_for_tracker</a>(
    tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a>,
    <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    client_order_id: u64
) {
    <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_garbage_collect">garbage_collect</a>(tracker);
    <b>let</b> account_order_id = new_account_client_order_id(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>), client_order_id);
    <b>if</b> (tracker.account_order_ids.contains(&account_order_id)) {
        // If the account_order_id already <b>exists</b> <b>with</b> a previously set expiration time,
        // we <b>update</b> the expiration time.
        <b>let</b> expiration_time = tracker.account_order_ids.remove(&account_order_id);
        <b>let</b> order_id_with_expiration =
            <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_ExpirationAndOrderId">ExpirationAndOrderId</a> { expiration_time, account_order_id };
        // If the mapping <b>exists</b>, then we remove the order ID <b>with</b> its expiration time.
        tracker.expiration_with_order_ids.remove(&order_id_with_expiration);
    };
    <b>let</b> current_time = velor_std::timestamp::now_microseconds();
    <b>let</b> expiration_time = current_time + tracker.pre_cancellation_window_secs;
    <b>let</b> order_id_with_expiration = <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_ExpirationAndOrderId">ExpirationAndOrderId</a> {
        expiration_time,
        account_order_id
    };
    tracker.account_order_ids.add(account_order_id, expiration_time);
    tracker.expiration_with_order_ids.add(order_id_with_expiration, <b>true</b>);
}
</code></pre>



</details>

<a id="0x7_pre_cancellation_tracker_is_pre_cancelled"></a>

## Function `is_pre_cancelled`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_is_pre_cancelled">is_pre_cancelled</a>(tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>, <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, client_order_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_is_pre_cancelled">is_pre_cancelled</a>(
    tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a>,
    <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    client_order_id: u64
): bool {
    <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_garbage_collect">garbage_collect</a>(tracker);
    <b>let</b> account_order_id = new_account_client_order_id(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>, client_order_id);
    <b>let</b> expiration_time_option = tracker.account_order_ids.get(&account_order_id);
    <b>if</b> (expiration_time_option.is_some()) {
        <b>let</b> current_time = velor_std::timestamp::now_seconds();
        <b>let</b> expiration_time = expiration_time_option.destroy_some();
        <b>if</b> (current_time &gt; expiration_time) {
            // This is possible <b>as</b> garbage collection may not be able <b>to</b> garbage collect all expired orders
            // in a single call.
            tracker.account_order_ids.remove(&account_order_id);
            <b>let</b> order_id_with_expiration =
                <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_ExpirationAndOrderId">ExpirationAndOrderId</a> { expiration_time, account_order_id };
            tracker.expiration_with_order_ids.remove(&order_id_with_expiration);
        } <b>else</b> {
            <b>return</b> <b>true</b>; // Order ID already <b>exists</b> <b>with</b> a valid expiration time.
        }
    };
    <b>return</b> <b>false</b>
}
</code></pre>



</details>

<a id="0x7_pre_cancellation_tracker_garbage_collect"></a>

## Function `garbage_collect`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_garbage_collect">garbage_collect</a>(tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">pre_cancellation_tracker::PreCancellationTracker</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>package</b>) <b>fun</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_garbage_collect">garbage_collect</a>(tracker: &<b>mut</b> <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_PreCancellationTracker">PreCancellationTracker</a>) {
    <b>let</b> i = 0;
    <b>let</b> current_time = velor_std::timestamp::now_seconds();
    <b>while</b> (i &lt; <a href="pre_cancellation_tracker.md#0x7_pre_cancellation_tracker_MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL">MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL</a>
        && !tracker.expiration_with_order_ids.is_empty()) {
        <b>let</b> (front_k, _) = tracker.expiration_with_order_ids.borrow_front();
        <b>if</b> (front_k.expiration_time &lt; current_time) {
            tracker.expiration_with_order_ids.pop_front();
            tracker.account_order_ids.remove(&front_k.account_order_id);
        } <b>else</b> {
            <b>break</b>;
        };
        i += 1;
    };
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
