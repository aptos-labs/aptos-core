module aptos_framework::user_func_wrapper {
    use aptos_framework::scheduled_txns::{Self, ScheduleMapKey};

    /// Called by the block executor when the scheduled transaction is run
    /// We need this wrapper function outside of the scheduled_txns module to prevent re-entrancy issues when a
    /// user_func() tries to (re)schedule another transaction
    fun execute_user_function(
        signer: signer, txn_key: ScheduleMapKey, block_timestamp_ms: u64
    ): bool {
        let txn_opt = scheduled_txns::get_txn_by_key(txn_key);
        if (txn_opt.is_none()) {
            return false
        };
        let txn = txn_opt.borrow();

        // Check if transaction has expired - if so, emit event and skip execution
        if (scheduled_txns::fail_txn_on_expired(txn, txn_key, block_timestamp_ms)) {
            // Transaction is expired - do not execute user function
            scheduled_txns::remove_txn_from_table(
                scheduled_txns::schedule_map_key_txn_id(&txn_key)
            );
            return true
        };

        if (scheduled_txns::is_scheduled_function_v1(txn)) {
            let f = scheduled_txns::get_scheduled_function_v1(txn);
            f();
        } else {
            if (scheduled_txns::fail_txn_on_invalid_auth_token(
                txn, txn_key, block_timestamp_ms
            )) {
                // Invalid auth token (expired or all scheduled txns canceled for the sender) - do not execute user func
            } else {
                let f = scheduled_txns::get_scheduled_function_v1_with_auth_token(txn);
                let updated_auth_token =
                    scheduled_txns::create_updated_auth_token_for_execution(txn);
                f(&signer, updated_auth_token);
            };
        };

        // Remove transaction from txn_table to enable proper refunding of storage gas fees
        scheduled_txns::remove_txn_from_table(
            scheduled_txns::schedule_map_key_txn_id(&txn_key)
        );
        true
    }

    #[test_only]
    public fun execute_user_function_test(
        signer: signer, txn_key: ScheduleMapKey, block_timestamp_ms: u64
    ): bool {
        execute_user_function(signer, txn_key, block_timestamp_ms)
    }
}
