#[test_only]
module aptos_framework::test_scheduled_txns {

    use std::signer;
    use aptos_framework::coin::{Self};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::scheduled_txns::{Self, ScheduleMapKey, mark_txn_to_remove_test};
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
        scheduled_txns::execute_user_function_wrapper_test(signer, key, block_timestamp_ms);
        //f(some<signer>(signer));
        // Finish execution
        mark_txn_to_remove_test(key);
    }

    #[persistent]
    fun simple_work_func(
        max_gas_amount: u64,
        max_gas_unit_price: u64,
        delta_time: u64
    ) {
        // do work without rescheduling (unsigned version can't reschedule)
        let _ = max_gas_amount + max_gas_unit_price + delta_time; // consume parameters
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
                foo
            );
        let gas_price_txn2 = 300;
        let work_foo = || simple_work_func(
            gas_price_txn2,
            txn_max_gas_units,
            3000
        );
        let txn2 =
            scheduled_txns::new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time,
                txn_max_gas_units,
                gas_price_txn2,
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
        scheduled_txns::remove_txns(timestamp::now_microseconds() / 1000);
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
}
