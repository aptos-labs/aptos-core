
<a id="0x1_user_func_wrapper"></a>

# Module `0x1::user_func_wrapper`



-  [Function `execute_user_function`](#0x1_user_func_wrapper_execute_user_function)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="scheduled_txns.md#0x1_scheduled_txns">0x1::scheduled_txns</a>;
</code></pre>



<a id="0x1_user_func_wrapper_execute_user_function"></a>

## Function `execute_user_function`

Called by the executor when the scheduled transaction is run


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

    <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_is_scheduled_function_v1">scheduled_txns::is_scheduled_function_v1</a>(txn)) {
        <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1">scheduled_txns::get_scheduled_function_v1</a>(txn);
        f();
    } <b>else</b> {
        // Validate auth token and cancel <b>if</b> invalid
        <b>if</b> (<a href="scheduled_txns.md#0x1_scheduled_txns_validate_and_cancel_if_invalid_auth_token">scheduled_txns::validate_and_cancel_if_invalid_auth_token</a>(txn, txn_key, block_timestamp_ms)) {
            <b>return</b> <b>true</b>
        };

        <b>let</b> f = <a href="scheduled_txns.md#0x1_scheduled_txns_get_scheduled_function_v1_with_auth_token">scheduled_txns::get_scheduled_function_v1_with_auth_token</a>(txn);
        <b>let</b> updated_auth_token = <a href="scheduled_txns.md#0x1_scheduled_txns_create_updated_auth_token_for_execution">scheduled_txns::create_updated_auth_token_for_execution</a>(txn);
        f(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, updated_auth_token);
    };

    <b>true</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
