
<a id="0x1_user_func_wrapper"></a>

# Module `0x1::user_func_wrapper`



-  [Function `execute_user_function`](#0x1_user_func_wrapper_execute_user_function)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="scheduled_txns.md#0x1_scheduled_txns">0x1::scheduled_txns</a>;
</code></pre>



<a id="0x1_user_func_wrapper_execute_user_function"></a>

## Function `execute_user_function`



<pre><code><b>fun</b> <a href="user_func_wrapper.md#0x1_user_func_wrapper_execute_user_function">execute_user_function</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: <a href="scheduled_txns.md#0x1_scheduled_txns_ScheduleMapKey">scheduled_txns::ScheduleMapKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="user_func_wrapper.md#0x1_user_func_wrapper_execute_user_function">execute_user_function</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, txn_key: ScheduleMapKey) {
    <b>let</b> txn = <a href="scheduled_txns.md#0x1_scheduled_txns_get_txn_by_key">scheduled_txns::get_txn_by_key</a>(txn_key).borrow();
    <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_is_scheduled_function_v1">scheduled_txns::is_scheduled_function_v1</a>(txn)) {
        <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1">scheduled_txns::get_scheduled_function_v1</a>(txn);
        f();
    } <b>else</b> {
        <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1_with_auth_token">scheduled_txns::get_scheduled_function_v1_with_auth_token</a>(txn);
        <b>let</b> auth_token = <a href="scheduled_txns.md#0x1_scheduled_txns_get_auth_token_from_txn">scheduled_txns::get_auth_token_from_txn</a>(txn);
        <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_allows_rescheduling">scheduled_txns::allows_rescheduling</a>(&auth_token)) {
            f(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, some(auth_token));
        } <b>else</b> {
            f(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, none&lt;ScheduledTxnAuthToken&gt;());
        };

    };

    <a href="scheduled_txns.md#0x1_scheduled_txns_remove_txn_from_table">scheduled_txns::remove_txn_from_table</a>(
        <a href="scheduled_txns.md#0x1_scheduled_txns_schedule_map_key_txn_id">scheduled_txns::schedule_map_key_txn_id</a>(&txn_key)
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
