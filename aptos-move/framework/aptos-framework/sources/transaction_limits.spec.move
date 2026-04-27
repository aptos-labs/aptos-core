spec aptos_framework::transaction_limits {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    spec validate_high_txn_limits {
        // TODO: set because of timeout — calls into stake, aptos_governance, and
        // delegation_pool across three match arms, which exceeds the 40s global limit.
        pragma verify = false;
    }
}
