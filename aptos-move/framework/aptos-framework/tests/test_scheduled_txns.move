#[test_only]
module aptos_framework::test_scheduled_txns {

    use std::signer;
    use std::string;
    use aptos_std::debug;
    use aptos_framework::user_func_wrapper;
    use aptos_framework::coin::{Self};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::scheduled_txns::{
        Self,
        ScheduleMapKey,
        mark_txn_to_remove_test,
        ScheduledTxnAuthToken,
        get_or_init_auth_num
    };

    const EXPIRY_DELTA_DEFAULT: u64 = 10 * 1000;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_validation;

    #[persistent]
    fun simpleFunc(_state: u64) {
        if (_state < 10) {
            _state = _state + 1;
        }
    }

    #[test_only]
    public fun mock_execute(key: ScheduleMapKey, signer: signer) {
        let block_timestamp_ms = timestamp::now_microseconds() / 1000;
        user_func_wrapper::execute_user_function_test(signer, key, block_timestamp_ms);
        // Finish execution
        mark_txn_to_remove_test(key);
    }

    #[persistent]
    fun simple_work_func(
        max_gas_amount: u64, max_gas_unit_price: u64, delta_time: u64
    ) {
        // do work without rescheduling (unsigned version can't reschedule)
        let _ = max_gas_amount + max_gas_unit_price + delta_time; // consume parameters
    }

    #[persistent]
    fun rescheduling_test_func(
        sender: &signer, auth_token: ScheduledTxnAuthToken
    ) {
        let current_time = timestamp::now_microseconds() / 1000;
        let next_schedule_time = current_time + 10000; // Would schedule 1 second later

        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| rescheduling_test_func(
                signer, auth_token
            );

        let txn =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                sender,
                auth_token,
                next_schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        scheduled_txns::insert(sender, txn);
    }

    #[persistent]
    fun user_func_with_auth_token(
        _signer: &signer, _auth_token: ScheduledTxnAuthToken
    ) {
        debug::print(&string::utf8(b"Running user func..."));
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_reschedule_with_reused_auth_token(
        fx: &signer, user: signer
    ) {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        scheduled_txns::setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000; // 1 second in future
        let expiration_time = schedule_time + 10000; // Token valid for 10 seconds

        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| rescheduling_test_func(
                signer, auth_token
            );

        let auth_token =
            scheduled_txns::create_mock_auth_token(true, expiration_time, sender_auth_num);
        let txn =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        let txn_key = scheduled_txns::insert(&user, txn);
        assert!(scheduled_txns::get_num_txns() == 1, scheduled_txns::get_num_txns());

        // Execute the transaction - it should work since allow_rescheduling = true
        user_func_wrapper::execute_user_function_test(user, txn_key, schedule_time
            + 100);
        assert!(scheduled_txns::get_num_txns() == 2, scheduled_txns::get_num_txns());
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65552)]
    fun test_disallow_reschedule_token(fx: &signer, user: signer) {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        scheduled_txns::setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000; // 1 second in future
        let expiration_time = schedule_time + 10000; // Token valid for 10 seconds

        // Test: Create auth token with allow_rescheduling = false
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| rescheduling_test_func(
                signer, auth_token
            );

        // Create auth token with allow_rescheduling = false
        let auth_token =
            scheduled_txns::create_mock_auth_token(false, expiration_time, sender_auth_num);
        let txn =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        // Insert the initial transaction
        let txn_key = scheduled_txns::insert(&user, txn);
        assert!(scheduled_txns::get_num_txns() == 1, scheduled_txns::get_num_txns());

        // Execute the transaction - it should work but with rescheduling disabled
        user_func_wrapper::execute_user_function_test(user, txn_key, schedule_time
            + 100);
    }

    // Purpose of this test is to test 'scheduled_txn_epilogue'
    #[test(fx = @0x1, user = @0x123)]
    fun test_scheduled_txn_gas_calculations(fx: &signer, user: signer) {
        let curr_mock_time_micro_s = 1000000;
        // Setup test environment
        scheduled_txns::setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Setup
        let fa_store_signer = scheduled_txns::get_deposit_owner_signer();
        let fa_store_addr = signer::address_of(&fa_store_signer);
        assert!(
            coin::balance<AptosCoin>(fa_store_addr) == 0,
            coin::balance<AptosCoin>(fa_store_addr)
        );

        let user_addr = signer::address_of(&user);
        let pre_balance = coin::balance<AptosCoin>(user_addr);

        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let txn_max_gas_units = 100;
        let gas_units_remaining = 50;

        // Create test transactions
        let foo = || simpleFunc(5);
        let gas_price_txn1 = 200;
        let txn1 =
            scheduled_txns::new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time,
                txn_max_gas_units,
                gas_price_txn1,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let gas_price_txn2 = 300;
        let work_foo = || simple_work_func(gas_price_txn2, txn_max_gas_units, 3000);
        let txn2 =
            scheduled_txns::new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time,
                txn_max_gas_units,
                gas_price_txn2,
                EXPIRY_DELTA_DEFAULT,
                work_foo
            );

        // Insert transactions
        let txn1_key = scheduled_txns::insert(&user, txn1);
        let txn2_key = scheduled_txns::insert(&user, txn2);
        // Check initial state
        assert!(scheduled_txns::get_num_txns() == 2, scheduled_txns::get_num_txns());

        // Check that deposit has been deducted from user account
        let balance_after_deposit = coin::balance<AptosCoin>(user_addr);
        let txn1_deposit = gas_price_txn1 * txn_max_gas_units;
        let txn2_deposit = gas_price_txn2 * txn_max_gas_units;
        assert!(
            (balance_after_deposit + txn1_deposit + txn2_deposit) == pre_balance,
            balance_after_deposit
        );

        // Move time forward and get ready transactions
        let ready_txns = scheduled_txns::get_ready_transactions_test(
            schedule_time + 1000
        );
        assert!(ready_txns.length() == 2, ready_txns.length());

        mark_txn_to_remove_test(txn1_key);
        // Execute and verify transaction epilogue
        let txn1_storage_fee_refund = 10;
        transaction_validation::scheduled_txn_epilogue_test_helper(
            &fa_store_signer,
            user_addr,
            txn1_key,
            txn1_storage_fee_refund,
            gas_price_txn1,
            txn_max_gas_units,
            gas_price_txn1 * txn_max_gas_units, // scheduling_deposit
            gas_units_remaining
        );

        let post_txn1_balance = coin::balance<AptosCoin>(user_addr);
        let txn1_deposit_refund =
            txn1_deposit - gas_price_txn1 * (txn_max_gas_units - gas_units_remaining);
        assert!(
            (balance_after_deposit + txn1_deposit_refund + txn1_storage_fee_refund)
                == post_txn1_balance,
            post_txn1_balance
        );

        // Cleanup
        let txn2_charged_gas_price = gas_price_txn2 - 10;
        let txn2_storage_fee_refund = 2000; // large refund, so that there is net refund
        transaction_validation::scheduled_txn_epilogue_test_helper(
            &fa_store_signer,
            user_addr,
            txn2_key,
            txn2_storage_fee_refund,
            txn2_charged_gas_price, // gas_price
            txn_max_gas_units,
            gas_price_txn2 * txn_max_gas_units, // scheduling_deposit
            gas_units_remaining
        );
        let post_txn2_balance = coin::balance<AptosCoin>(user_addr);
        let txn2_deposit_refund =
            txn2_deposit
                - txn2_charged_gas_price * (txn_max_gas_units - gas_units_remaining);
        assert!(
            (post_txn1_balance + txn2_deposit_refund + txn2_storage_fee_refund)
                == post_txn2_balance,
            post_txn2_balance
        );

        // check execution without rescheduling
        mock_execute(txn2_key, user);
        scheduled_txns::remove_txns();
        assert!(scheduled_txns::get_num_txns() == 0, scheduled_txns::get_num_txns());
        // Shutdown should cancel all transactions and refund all deposits
        scheduled_txns::shutdown_test(fx);
        scheduled_txns::continue_shutdown_test(100);

        // Check that deposit store has been emptied
        assert!(
            coin::balance<AptosCoin>(fa_store_addr) == 0,
            coin::balance<AptosCoin>(fa_store_addr)
        );
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_cancel_all_with_auth_tokens(fx: &signer, user: signer) {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        scheduled_txns::setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 2000;
        let expiration_time = schedule_time + 10000;

        // Create multiple transactions with auth tokens
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| user_func_with_auth_token(
                signer, auth_token
            );

        // Create 3 transactions with the same auth num
        let auth_token =
            scheduled_txns::create_mock_auth_token(true, expiration_time, sender_auth_num);
        let txn1 =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let txn2 =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time + 1000,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let txn3 =
            scheduled_txns::new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time + 2000,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        // Store transaction keys for later testing
        let txn1_key = scheduled_txns::insert(&user, txn1);
        let _txn2_key = scheduled_txns::insert(&user, txn2);
        let _txn3_key = scheduled_txns::insert(&user, txn3);
        assert!(scheduled_txns::get_num_txns() == 3, scheduled_txns::get_num_txns());

        // Cancel all - this increments the sender's auth num
        scheduled_txns::cancel_all_test(&user);

        // Verify the sender auth num was incremented
        let new_auth_num = get_or_init_auth_num(user_addr);
        assert!(new_auth_num == sender_auth_num + 1, new_auth_num);

        // The transactions are still in the queue because cancellation is lazy
        assert!(scheduled_txns::get_num_txns() == 3, scheduled_txns::get_num_txns());

        // Test lazy delete: try to execute one of the transactions; should not execute; verify by checking the logging
        // in user_func_with_auth_token
        let current_time = schedule_time;
        user_func_wrapper::execute_user_function_test(user, txn1_key, current_time);
    }
}
