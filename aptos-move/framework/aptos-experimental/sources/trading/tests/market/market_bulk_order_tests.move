#[test_only]
module aptos_experimental::market_bulk_order_tests {
    use std::option;
    use std::signer;
    use aptos_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
    };
    use aptos_experimental::market_test_utils::{
        place_taker_order_and_verify_fill,
    };
    use aptos_experimental::event_utils;
    use aptos_experimental::order_book_types::{good_till_cancelled};
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
        test_self_matching_allowed_helper,
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

        let bulk_order_id_opt = place_bulk_order(
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
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
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
        verify_orders_cleanup(&market, bulk_order_id, taker_order_id, false, true);

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

    #[test(
        aptos_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_allowed(
        aptos_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_self_matching_allowed_helper(aptos_framework, admin, market_signer, maker1, maker2, true);
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
        test_taker_partial_cancelled_maker_reinserted_helper(admin, market_signer, maker, taker, true);
    }

    #[test(admin = @0x1, market_signer = @0x123, maker = @0x456)]
    public fun test_bulk_order_cancellation_event_fields(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
    ) {
        use aptos_experimental::market_types::{BulkOrderModifiedEvent, verify_bulk_order_modified_event};
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
        let order_id_option = place_bulk_order(
            &mut market,
            maker,
            vector[bid_price],
            vector[bid_size],
            vector[ask_price],
            vector[ask_size],
        );

        // Verify order was placed successfully
        assert!(order_id_option.is_some(), 0);
        let order_id = order_id_option.destroy_some();

        // Cancel the bulk order
        cancel_bulk_order(&mut market, maker, market_types::order_cancellation_reason_cancelled_by_user(), &test_market_callbacks());

        // Verify the cancellation event
        let cancel_event_store = new_event_store();
        let events = latest_emitted_events<BulkOrderModifiedEvent>(&mut cancel_event_store, option::some(1));
        assert!(events.length() == 1, 1);

        let bulk_order_cancelled_event = events[0];

        // Verify that the event contains the correct cancelled order details
        verify_bulk_order_modified_event(
            bulk_order_cancelled_event,
            order_id,
            sequence_number,
            market.get_market_address(),
            maker_addr,
            vector[], // bid_prices - empty after cancellation
            vector[], // bid_sizes - empty after cancellation
            vector[], // ask_prices - empty after cancellation
            vector[], // ask_sizes - empty after cancellation
            vector[bid_price], // cancelled_bid_prices - the original bid PRICE (100)
            vector[bid_size], // cancelled_bid_sizes - the original bid SIZE (50)
            vector[ask_price], // cancelled_ask_prices - the original ask PRICE (110)
            vector[ask_size], // cancelled_ask_sizes - the original ask SIZE (60)
            sequence_number, // previous_seq_num - same as sequence_number for cancellation
        );

        market.destroy_market();
    }
}
