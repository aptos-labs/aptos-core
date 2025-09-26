module aptos_framework::user_func_wrapper {
    use aptos_framework::scheduled_txns::{Self, ScheduleMapKey, ScheduledTxnAuthToken};
    use std::option::{some, none};

    fun execute_user_function(signer: signer, txn_key: ScheduleMapKey) {
        let txn = scheduled_txns::get_txn_by_key(txn_key).borrow();
        if (scheduled_txns::is_scheduled_function_v1(txn)) {
            let f = scheduled_txns::get_scheduled_function_v1(txn);
            f();
        } else {
            let f = scheduled_txns::get_scheduled_function_v1_with_auth_token(txn);
            let auth_token = scheduled_txns::get_auth_token_from_txn(txn);
            if (scheduled_txns::allows_rescheduling(&auth_token)) {
                f(&signer, some(auth_token));
            } else {
                f(&signer, none<ScheduledTxnAuthToken>());
            };

        };

        scheduled_txns::remove_txn_from_table(
            scheduled_txns::schedule_map_key_txn_id(&txn_key)
        );
    }

    #[test_only]
    public fun execute_user_function_test(
        signer: signer, txn_key: ScheduleMapKey
    ) {
        execute_user_function(signer, txn_key)
    }
}
