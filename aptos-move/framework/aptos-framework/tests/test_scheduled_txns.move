#[test_only]
module aptos_framework::test_scheduled_txns {
    use std::signer;
    use aptos_framework::coin::{Self};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::scheduled_txns::{Self, create_transaction_id};
    use aptos_framework::transaction_validation;

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

        // Create permissioned handle and set permissions
        let storable_perm_handle = scheduled_txns::setup_permissions(&user);

        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let txn_max_gas_units = 100;
        let gas_units_remaining = 50;

        // Create test transactions
        let gas_price_txn1 = 20;
        let txn1 =
            scheduled_txns::create_scheduled_txn(
                storable_perm_handle,
                schedule_time,
                gas_price_txn1,
                txn_max_gas_units,
                0
            );
        let gas_price_txn2 = 30;
        let txn2 =
            scheduled_txns::create_scheduled_txn(
                storable_perm_handle,
                schedule_time,
                gas_price_txn2,
                txn_max_gas_units,
                2000
            );

        // Insert transactions
        let txn1_id = scheduled_txns::insert(&user, txn1);
        let tId1 = create_transaction_id(txn1_id);
        let txn2_id = scheduled_txns::insert(&user, txn2);
        let tId2 = create_transaction_id(txn2_id);

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
        let ready_txns =
            scheduled_txns::get_ready_transactions_test(schedule_time + 1000, 10);
        assert!(ready_txns.length() == 2, ready_txns.length());

        // Execute and verify transaction epilogue
        let txn1_storage_fee_refund = 10;
        transaction_validation::scheduled_txn_epilogue_test_helper(
            &fa_store_signer,
            user_addr,
            tId1,
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
            tId2,
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

        // check reschedule
        scheduled_txns::remove_txns();
        assert!(scheduled_txns::get_num_txns() == 1, scheduled_txns::get_num_txns());

        // Shutdown should cancel all transactions and refund all deposits
        scheduled_txns::shutdown_test(fx);

        // Check that deposit store has been emptied
        assert!(
            coin::balance<AptosCoin>(fa_store_addr) == 0,
            coin::balance<AptosCoin>(fa_store_addr)
        );
    }
}
