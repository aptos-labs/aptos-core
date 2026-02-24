#[test_only]
module aptos_experimental::market_bulk_order_tests {
    use std::option;
    use std::signer;
    use aptos_trading::order_book_types::{good_till_cancelled};
    use aptos_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
        place_bulk_order_callback_called,
        get_place_bulk_order_callback_data
    };
    use aptos_experimental::market_test_utils::{place_taker_order_and_verify_fill};
    use aptos_experimental::event_utils;
    use aptos_experimental::market_types;

    // Import common functions from market_tests
    use aptos_experimental::market_tests_common::{
        setup_market,
        place_bulk_order,
        verify_positions,
        verify_orders_cleanup,
        test_order_full_match,
        test_order_partial_match,
        test_order_no_match,
        test_gtc_taker_fully_filled_internal,
        test_gtc_taker_partially_filled_helper,
        test_post_only_success_helper,
        test_post_only_failure_helper,
        test_taker_partial_cancelled_maker_reinserted_helper,
        test_self_matching_allowed_helper
    };

    const PRE_CANCEL_WINDOW_MICROS: u64 = 1000000; // 1 second

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc_taker_fully_filled_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        test_gtc_taker_fully_filled_internal(&mut market, maker, taker, true);
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_bulk_orders_multiple_levels(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        // Place bulk orders with multiple price levels
        let bid_prices = vector[1000, 990, 980];
        let bid_sizes = vector[1000000, 1500000, 2000000];
        let ask_prices = vector[1010, 1020, 1030];
        let ask_sizes = vector[800000, 1200000, 1600000];

        let bulk_order_id_opt =
            place_bulk_order(
                &mut market,
                maker,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes
            );

        // Verify bulk order was placed
        assert!(bulk_order_id_opt.is_some(), 1);
        let bulk_order_id = bulk_order_id_opt.destroy_some();

        // Verify positions are updated
        verify_positions(maker_addr, taker_addr, 0, 0);

        // Place a taker order that matches one of the ask orders
        let (taker_order_id, _) =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                1010,
                800000,
                true, // is_bid
                good_till_cancelled(),
                vector[800000],
                vector[1010],
                maker_addr,
                vector[bulk_order_id],
                vector[option::none()],
                vector[800000 + 1200000 + 1600000],
                vector[800000 + 1200000 + 1600000],
                &mut event_store,
                false,
                option::none(),
                new_test_order_metadata(1),
                true,
                &test_market_callbacks()
            );

        // Verify positions after match
        verify_positions(maker_addr, taker_addr, 800000, 800000);

        // Verify order cleanup
        verify_orders_cleanup(
            &market,
            bulk_order_id,
            taker_order_id,
            false,
            true
        );

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc_taker_partially_filled_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_gtc_taker_partially_filled_helper(admin, market_signer, maker, taker, true);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_post_only_success_bulk(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_post_only_success_helper(admin, market_signer, maker1, maker2, true);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_post_only_failure_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_post_only_failure_helper(admin, market_signer, maker, taker, true);
    }

    #[
        test(
            aptos_framework = @0x1,
            admin = @0x1,
            market_signer = @0x123,
            maker1 = @0x456,
            maker2 = @0x789
        )
    ]
    public fun test_self_matching_allowed(
        aptos_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_self_matching_allowed_helper(
            aptos_framework,
            admin,
            market_signer,
            maker1,
            maker2,
            true
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_full_match_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_full_match(admin, market_signer, maker, taker, false, true);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_market_order_full_match_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_full_match(
            admin,
            market_signer,
            maker,
            taker,
            true, // is_market_order
            true
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_partial_match_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_partial_match(
            admin,
            market_signer,
            maker,
            taker,
            false, // is_market_order
            true
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_market_order_partial_match_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_partial_match(
            admin,
            market_signer,
            maker,
            taker,
            true, // is_market_order
            true
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_no_match_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_no_match(
            admin,
            market_signer,
            maker,
            taker,
            false, // is_market_order
            true
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_taker_partial_cancelled_maker_reinserted_bulk(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_taker_partial_cancelled_maker_reinserted_helper(
            admin, market_signer, maker, taker, true
        );
    }

    #[test(admin = @0x1, market_signer = @0x123, maker = @0x456)]
    public fun test_bulk_order_cancellation_event_fields(
        admin: &signer,
        market_signer: &signer,
        maker: &signer
    ) {
        use aptos_experimental::market_types::BulkOrderModifiedEvent;
        use aptos_experimental::market_bulk_order::cancel_bulk_order;
        use aptos_experimental::event_utils::{new_event_store, latest_emitted_events};

        let market = setup_market(admin, market_signer);
        let maker_addr = signer::address_of(maker);

        // Define the bulk order parameters
        let bid_price = 100u64;
        let bid_size = 50u64;
        let ask_price = 110u64;
        let ask_size = 60u64;
        let sequence_number = 1u64; // The place_bulk_order helper uses sequence_number = 1 internally

        // Place bulk order with one bid and one ask
        let order_id_option =
            place_bulk_order(
                &mut market,
                maker,
                vector[bid_price],
                vector[bid_size],
                vector[ask_price],
                vector[ask_size]
            );

        // Verify order was placed successfully
        assert!(order_id_option.is_some(), 0);
        let order_id = order_id_option.destroy_some();

        // Cancel the bulk order
        cancel_bulk_order(
            &mut market,
            maker,
            market_types::order_cancellation_reason_cancelled_by_user(),
            &test_market_callbacks()
        );

        // Verify the cancellation event
        let cancel_event_store = new_event_store();
        let events =
            latest_emitted_events<BulkOrderModifiedEvent>(
                &mut cancel_event_store, option::some(1)
            );
        assert!(events.length() == 1, 1);

        let bulk_order_cancelled_event = events[0];

        // Verify that the event contains the correct cancelled order details
        bulk_order_cancelled_event.verify_bulk_order_modified_event(
            order_id,
            sequence_number,
            market.get_market_address(),
            maker_addr,
            vector[],
            vector[],
            vector[],
            vector[],
            vector[bid_price],
            vector[bid_size],
            vector[ask_price],
            vector[ask_size],
            sequence_number
        );

        market.destroy_market();
    }

    #[test(admin = @0x1, market_signer = @0x123, maker = @0x456)]
    public fun test_place_bulk_order_callback(
        admin: &signer,
        market_signer: &signer,
        maker: &signer
    ) {
        use aptos_experimental::market_bulk_order;

        let market = setup_market(admin, market_signer);
        let maker_addr = signer::address_of(maker);

        // Verify callback has not been called yet
        assert!(!place_bulk_order_callback_called(maker_addr), 0);

        // Place initial bulk order with sequence number 1
        let bid_prices = vector[100u64, 90u64];
        let bid_sizes = vector[50u64, 60u64];
        let ask_prices = vector[110u64, 120u64];
        let ask_sizes = vector[70u64, 80u64];

        let order_id_option =
            market_bulk_order::place_bulk_order(
                &mut market,
                maker_addr,
                1, // sequence number
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_order_metadata(1),
                &test_market_callbacks()
            );
        assert!(order_id_option.is_some(), 1);
        let order_id = order_id_option.destroy_some();

        // Verify callback was called with correct parameters
        assert!(place_bulk_order_callback_called(maker_addr), 2);
        let (
            callback_order_id,
            callback_bid_prices,
            callback_bid_sizes,
            callback_ask_prices,
            callback_ask_sizes,
            callback_cancelled_bid_prices,
            callback_cancelled_bid_sizes,
            callback_cancelled_ask_prices,
            callback_cancelled_ask_sizes
        ) = get_place_bulk_order_callback_data(maker_addr);
        assert!(callback_order_id == order_id, 3);
        assert!(callback_bid_prices == bid_prices, 4);
        assert!(callback_bid_sizes == bid_sizes, 5);
        assert!(callback_ask_prices == ask_prices, 6);
        assert!(callback_ask_sizes == ask_sizes, 7);
        // For fresh order placement, cancelled vectors should be empty
        assert!(callback_cancelled_bid_prices == vector[], 8);
        assert!(callback_cancelled_bid_sizes == vector[], 9);
        assert!(callback_cancelled_ask_prices == vector[], 10);
        assert!(callback_cancelled_ask_sizes == vector[], 11);

        // Place a replacement bulk order with sequence number 2
        let new_bid_prices = vector[100u64];
        let new_bid_sizes = vector[55u64];
        let new_ask_prices = vector[115u64, 125u64];
        let new_ask_sizes = vector[75u64, 85u64];

        let new_order_id_option =
            market_bulk_order::place_bulk_order(
                &mut market,
                maker_addr,
                2, // sequence number - must be higher than previous
                new_bid_prices,
                new_bid_sizes,
                new_ask_prices,
                new_ask_sizes,
                new_test_order_metadata(2),
                &test_market_callbacks()
            );
        assert!(new_order_id_option.is_some(), 12);
        let new_order_id = new_order_id_option.destroy_some();

        // Verify callback was called with correct parameters for replacement order
        let (
            callback_order_id,
            callback_bid_prices,
            callback_bid_sizes,
            callback_ask_prices,
            callback_ask_sizes,
            _callback_cancelled_bid_prices,
            _callback_cancelled_bid_sizes,
            _callback_cancelled_ask_prices,
            _callback_cancelled_ask_sizes
        ) = get_place_bulk_order_callback_data(maker_addr);
        assert!(callback_order_id == new_order_id, 13);
        assert!(callback_bid_prices == new_bid_prices, 14);
        assert!(callback_bid_sizes == new_bid_sizes, 15);
        assert!(callback_ask_prices == new_ask_prices, 16);
        assert!(callback_ask_sizes == new_ask_sizes, 17);

        market.destroy_market();
    }
}
