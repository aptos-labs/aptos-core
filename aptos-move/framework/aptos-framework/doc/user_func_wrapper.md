
<a id="0x1_user_func_wrapper"></a>

# Module `0x1::user_func_wrapper`



-  [Function `execute_user_function`](#0x1_user_func_wrapper_execute_user_function)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="scheduled_txns.md#0x1_scheduled_txns">0x1::scheduled_txns</a>;
</code></pre>



<a id="0x1_user_func_wrapper_execute_user_function"></a>

## Function `execute_user_function`

Called by the block executor when the scheduled transaction is run
We need this wrapper function outside of the scheduled_txns module to prevent re-entrancy issues when a
user_func() tries to (re)schedule another transaction


<pre><code><b>fun</b> <a href="user_func_wrapper.md#0x1_user_func_wrapper_execute_user_function">execute_user_function</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>, block_timestamp_ms: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="user_func_wrapper.md#0x1_user_func_wrapper_execute_user_function">execute_user_function</a>(
    <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: ScheduleMapKey, block_timestamp_ms: u64
): bool {
    <b>let</b> txn_opt = <a href="scheduled_txns.md#0x1_scheduled_txns_get_txn_by_key">scheduled_txns::get_txn_by_key</a>(txn_key);
    <b>if</b> (txn_opt.is_none()) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> txn = txn_opt.borrow();

    // Check <b>if</b> transaction <b>has</b> expired - <b>if</b> so, emit <a href="event.md#0x1_event">event</a> and skip execution
    <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_fail_txn_on_expired">scheduled_txns::fail_txn_on_expired</a>(txn, txn_key, block_timestamp_ms)) {
        // Transaction is expired - do not execute user function
        <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txn_from_table">scheduled_txns::remove_txn_from_table</a>(
            <a href="scheduled_txns.md#0x1_scheduled_txns_schedule_map_key_txn_id">scheduled_txns::schedule_map_key_txn_id</a>(&txn_key)
        );
        <b>return</b> <b>true</b>
    };

    <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_is_scheduled_function_v1">scheduled_txns::is_scheduled_function_v1</a>(txn)) {
        <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1">scheduled_txns::get_scheduled_function_v1</a>(txn);
        f();
    } <b>else</b> {
        <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_fail_txn_on_invalid_auth_token">scheduled_txns::fail_txn_on_invalid_auth_token</a>(
            txn, txn_key, block_timestamp_ms
        )) {
            // Invalid auth token (expired or all scheduled txns canceled for the sender) - do not execute user func
        } <b>else</b> {
            <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1_with_auth_token">scheduled_txns::get_scheduled_function_v1_with_auth_token</a>(txn);
            <b>let</b> updated_auth_token =
                <a href="scheduled_txns.md#0x1_scheduled_txns_create_updated_auth_token_for_execution">scheduled_txns::create_updated_auth_token_for_execution</a>(txn);
            f(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, updated_auth_token);
        };
    };

    // Remove transaction from txn_table <b>to</b> enable proper refunding of storage gas fees
    <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txn_from_table">scheduled_txns::remove_txn_from_table</a>(
        <a href="scheduled_txns.md#0x1_scheduled_txns_schedule_map_key_txn_id">scheduled_txns::schedule_map_key_txn_id</a>(&txn_key)
    );
    <b>true</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
