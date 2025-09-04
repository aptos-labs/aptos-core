#[test_only]
module velor_experimental::market_single_order_tests {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use velor_framework::timestamp;
    use velor_experimental::event_utils::latest_emitted_events;
    use velor_experimental::clearinghouse_test;
    use velor_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
        get_position_size,
    };
    use velor_experimental::market_test_utils::{
        place_order_and_verify,
        place_taker_order_and_verify_fill,
        place_taker_order,
        verify_cancel_event,
    };
    use velor_experimental::event_utils;
    use velor_experimental::market_types::{order_status_open};
    use velor_experimental::market::{OrderEvent};
    use velor_experimental::order_book_types::OrderIdType;
    use velor_experimental::order_book_types::{good_till_cancelled};

    // Import common functions from market_tests
    use velor_experimental::market_tests_common::{
        setup_market,
        place_maker_order,
        verify_positions,
        test_order_full_match,
        test_order_partial_match,
        test_order_no_match,
        test_taker_partial_cancelled_maker_reinserted_helper,
        test_gtc_taker_fully_filled_internal,
        test_gtc_taker_partially_filled_helper,
        test_post_only_success_helper,
        test_post_only_failure_helper, test_self_matching_not_allowed_helper, test_self_matching_allowed_helper,
    };

    const PRE_CANCEL_WINDOW_MICROS: u64 = 1000000; // 1 second

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc_taker_fully_filled(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        test_gtc_taker_fully_filled_internal(&mut market, maker, taker, false);
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc_taker_partially_filled(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_gtc_taker_partially_filled_helper(admin, market_signer, maker, taker, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_post_only_success(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_post_only_success_helper(admin, market_signer, maker1, maker2, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_post_only_failure(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_post_only_failure_helper(admin, market_signer, maker, taker, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_full_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_order_full_match(admin, market_signer, maker, taker, false, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_market_order_full_match(
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
            false
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_partial_match(
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
            false
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_market_order_partial_match(
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
            false
        );
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_ioc_no_match(
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
            false
        );
    }

    #[test(admin = @0x1, market_signer = @0x123, taker = @0x789)]
    public fun test_market_order_empty_order_book(
        admin: &signer, market_signer: &signer, taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        market.place_market_order(
            taker,
            1000000,
            false, // is_buy
            new_test_order_metadata(1),
                option::none(), // client_order_id
            1000,
            true,
                &test_market_callbacks(),
        );

        let events = latest_emitted_events<OrderEvent>(&mut event_store, option::some(1));
        let order_place_event = events[0];
        let order_id = order_place_event.get_order_id_from_event();
        order_place_event.verify_order_event(
            order_id,
            option::none(), // client_order_id
            market.get_market(),
            signer::address_of(taker),
            1000000,
            1000000,
            1000000,
            1, // price
            false,
            false, // Even if it's a market order, it won't cross.
            order_status_open()
        );
        verify_cancel_event(
            &mut market,
            taker,
            false, // Not a maker order
            order_id,
            option::none(), // client_order_id
            1, // price
            1000000, // original size
            0, // filled size
            1000000, // remaining size
            false, // Order is cancelled
            &mut event_store
        );
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_taker_order_partial_fill(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        // Place maker order
        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000, // price
            500000, // 0.5 BTC
            true,
            &mut event_store,
            false,
        );

        // Taker order that will fully consume maker order but still have remaining size
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1000,
            1000000, // 1 BTC
            false, // is_bid
            good_till_cancelled(),
            vector[500000], // 0.5 BTC
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[500000],
            vector[500000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        // Check positions after fill
        verify_positions(maker_addr, taker_addr, 500000, 500000);

        // Verify maker order fully filled
        assert!(market.get_remaining_size(maker_order_id) == 0);
        assert!(!clearinghouse_test::order_exists(maker_order_id));

        // Taker order partially filled
        assert!(
            market.get_remaining_size(taker_order_id) == 500000 // 0.5 BTC remaining
        );
        assert!(clearinghouse_test::order_exists(taker_order_id));

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_taker_order_multiple_fills(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        // Place several maker order with small sizes.
        let i = 1;
        let maker_order_ids = vector::empty<OrderIdType>();
        let expected_fill_sizes = vector::empty<u64>();
        let fill_prices = vector::empty<u64>();
        let maker_orig_sizes = vector::empty<u64>();
        let maker_client_order_ids = vector::empty<Option<u64>>();
        while (i < 6) {
            let maker_order_id = place_maker_order(
                &mut market,
                maker,
                1000 - i,
                10000 * i,
                true,
                &mut event_store,
                false
            );
            maker_order_ids.push_back(maker_order_id);
            maker_client_order_ids.push_back(option::none());
            expected_fill_sizes.push_back(10000 * i);
            maker_orig_sizes.push_back(10000 * i);
            fill_prices.push_back(1000 - i);
            i += 1;
        };
        let total_fill_size = expected_fill_sizes.fold(0, |acc, x| acc + x);

        // Order not matched yet, so the balance should not change
        verify_positions(maker_addr, taker_addr, 0, 0);
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            990,
            1000000,
            false,
            good_till_cancelled(),
            expected_fill_sizes,
            fill_prices,
            maker_addr,
            maker_order_ids,
            maker_client_order_ids,
            maker_orig_sizes,
            maker_orig_sizes,
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );
        verify_positions(maker_addr, taker_addr, total_fill_size, total_fill_size);
        // Ensure all maker orders are cleaned up
        while (maker_order_ids.length() > 0) {
            let maker_order_id = maker_order_ids.pop_back();
            assert!(!clearinghouse_test::order_exists(maker_order_id));
        };
        // Taker order should not be cleaned up since it is partially filled
        assert!(clearinghouse_test::order_exists(taker_order_id));
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_taker_partial_cancelled_maker_reinserted(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        test_taker_partial_cancelled_maker_reinserted_helper(admin, market_signer, maker, taker, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_not_allowed(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_self_matching_not_allowed_helper(admin, market_signer, maker1, maker2, false);
    }

    #[test(
       velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_allowed(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        test_self_matching_allowed_helper(velor_framework, admin, market_signer, maker1, maker2, false);
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_not_allowed_no_match(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);
        let event_store = event_utils::new_event_store();

        let maker1_order_id = place_maker_order(
            &mut market,
            maker1,
            1001,
            2000000,
            true,
            &mut event_store,
            false,
        );

        let _ = place_maker_order(
            &mut market,
            maker2,
            1000,
            2000000,
            true,
            &mut event_store,
            false
        );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker1_addr) == 0);

        // This should result in a self match order which should be cancelled and the taker order should not match
        place_taker_order(
            &mut market,
            maker1,
            option::none(),
            option::some(1001),
            1000000,
            false,
            good_till_cancelled(),
            &mut event_store,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        verify_cancel_event(
            &mut market,
            maker1,
            false,
            maker1_order_id,
            option::none(),
            1001,
            2000000,
            0,
            2000000,
            true,
            &mut event_store
        );

        verify_positions(maker1_addr, maker2_addr, 0, 0);
        market.destroy_market()
    }

    #[test(velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456)]
    public fun test_duplicate_client_order_id_not_allowed(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer
    ) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();

        let _ = place_order_and_verify(
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
            option::some(111),
            &test_market_callbacks()
        );

        let _ = place_order_and_verify(
            &mut market,
            maker1,
            option::some(1000),
            2000000,
            true,
            good_till_cancelled(),
            &mut event_store,
            false,
            true, // This should fail due to duplicate client order ID
            new_test_order_metadata(1),
            option::some(111), // Duplicate client order ID
            &test_market_callbacks()
        );
        market.destroy_market()
    }

    #[test(velor_framework = @0x1, admin = @0x1, market_signer = @0x123, maker1 = @0x456)]
    public fun test_metadata_update(
        velor_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer
    ) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();

        let order_id = place_order_and_verify(
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
            option::some(111),
            &test_market_callbacks()
        );

        let metadata = market.get_order_metadata_by_client_id(signer::address_of(maker1), 111);
        assert!(metadata.destroy_some() == new_test_order_metadata(1));

        // Test getting the metadata by order ID
        let metadata_by_order_id = market.get_order_metadata(order_id);
        assert!(metadata_by_order_id.destroy_some() == new_test_order_metadata(1));

        // Update metadata
        market.set_order_metadata_by_client_id(
            signer::address_of(maker1),
            111,
            new_test_order_metadata(2)
        );

        // Verify updated metadata
        let updated_metadata = market.get_order_metadata_by_client_id(signer::address_of(maker1), 111);
        assert!(updated_metadata.destroy_some() == new_test_order_metadata(2));

        // Update metadata by order ID
        market.set_order_metadata(order_id, new_test_order_metadata(3));
        let updated_metadata_by_order_id = market.get_order_metadata(order_id);
        assert!(updated_metadata_by_order_id.destroy_some() == new_test_order_metadata(3));
        market.destroy_market()
    }
}
