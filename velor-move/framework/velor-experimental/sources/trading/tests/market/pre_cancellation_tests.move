#[test_only]
module velor_experimental::pre_cancellation_tests {
    use std::option;
    use velor_framework::timestamp;
    use velor_experimental::order_book_types::good_till_cancelled;
    use velor_experimental::clearinghouse_test;
    use velor_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
    };
    use velor_experimental::market_test_utils::{
        place_order_and_verify, verify_cancel_event,
    };
    use velor_experimental::event_utils;
    use velor_experimental::market::{new_market, new_market_config};

    const PRE_CANCEL_WINDOW_SECS: u64 = 1; // 1 second

    #[test(velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456)]
    public fun test_pre_cancellation_success(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer
    ) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true, PRE_CANCEL_WINDOW_SECS)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        market.cancel_order_with_client_id(maker1, 1000, &test_market_callbacks());
        let _ =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1001),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                true, // Order should be cancelled as it was pre-cancelled
                new_test_order_metadata(1),
                option::some(1000),
                &test_market_callbacks()
            );
        // Place another order with same client order ID and verify that it is also cancelled
        let _ =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1001),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                true, // Order should be cancelled as it was pre-cancelled
                new_test_order_metadata(1),
                option::some(1000),
                &test_market_callbacks()
            );

        // Place an order with a different client order ID and verify that it is not cancelled
        let _ =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1002),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false, // Order should not be cancelled
                new_test_order_metadata(1),
                option::some(1002),
                &test_market_callbacks()
            );
        market.destroy_market()
    }

    #[test(velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456)]
    public fun test_pre_cancellation_after_order_placement(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer
    ) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true, PRE_CANCEL_WINDOW_SECS)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let order_id =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1001),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(1),
                option::some(1000),
                &test_market_callbacks()
            );
        assert!(market.get_remaining_size(order_id) == 2000000);
        // Pre-cancel the order after it has been placed
        market.cancel_order_with_client_id(maker1, 1000, &test_market_callbacks());
        verify_cancel_event(
            &mut market,
            maker1,
            false, // Not a maker order
            order_id,
            option::some(1000),
            1001,
            2000000,
            0,
            2000000,
            true, // Order is cancelled
            &mut event_store
        );
        assert!(market.get_remaining_size(order_id) == 0);
        market.destroy_market();
    }

    #[test(velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456)]
    public fun test_pre_cancellation_after_expiration(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer
    ) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true, PRE_CANCEL_WINDOW_SECS)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        market.cancel_order_with_client_id(maker1, 1000, &test_market_callbacks());
        let _ =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1001),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                true, // Order should be cancelled as it was pre-cancelled
                new_test_order_metadata(1),
                option::some(1000),
                &test_market_callbacks()
            );
        let initial_time = timestamp::now_seconds();
        timestamp::update_global_time_for_test_secs(initial_time + 5); // 5 seconds later
        // Should be considered pre-cancelled before expiration
        // Place another order with same client order ID and verify that it is not cancelled
        let _ =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1001),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false, // Order should not be cancelled as it was pre-cancelled after expiration
                new_test_order_metadata(1),
                option::some(1000),
                &test_market_callbacks()
            );
        market.destroy_market()
    }
}
