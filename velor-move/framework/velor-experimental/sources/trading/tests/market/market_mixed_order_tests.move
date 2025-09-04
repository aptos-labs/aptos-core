#[test_only]
module velor_experimental::market_mixed_order_tests {
    use std::option;
    use std::signer;
    use velor_experimental::clearinghouse_test;
    use velor_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
        get_position_size,
    };
    use velor_experimental::market_test_utils::{
        place_taker_order_and_verify_fill,
    };
    use velor_experimental::event_utils;
    use velor_experimental::order_book_types::{good_till_cancelled};

    // Import common functions from market_tests
    use velor_experimental::market_tests_common::{
        setup_market,
        place_maker_order,
        place_bulk_order,
        verify_positions,
        verify_orders_cleanup,
        verify_bulk_order_cleanup
    };

    const PRE_CANCEL_WINDOW_MICROS: u64 = 1000000; // 1 second

    #[test(
        admin = @0x1, market_signer = @0x123, bulk_maker = @0x456, single_maker = @0x789, taker = @0xabc
    )]
    public fun test_bulk_and_single_orders_coexist(
        admin: &signer,
        market_signer: &signer,
        bulk_maker: &signer,
        single_maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let bulk_maker_addr = signer::address_of(bulk_maker);
        let single_maker_addr = signer::address_of(single_maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        // Place bulk orders
        let bid_prices = vector[1000, 990];
        let bid_sizes = vector[1000000, 1500000];
        let ask_prices = vector[1010, 1020];
        let ask_sizes = vector[800000, 1200000];

        let bulk_order_id_opt = place_bulk_order(
            &mut market,
            bulk_maker,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // Verify bulk order was placed
        assert!(bulk_order_id_opt.is_some(), 1);
        let _bulk_order_id = bulk_order_id_opt.destroy_some();

        // Place a single order
        let single_order_id = place_maker_order(
            &mut market,
            single_maker,
            1005,
            1000000,
            true, // is_bid = true (bid)
            &mut event_store,
            false // not bulk
        );

        // Verify both orders exist
        assert!(clearinghouse_test::bulk_order_exists(bulk_maker_addr));
        assert!(clearinghouse_test::order_exists(single_order_id));

        assert!(market.get_remaining_size(single_order_id) == 1000000);
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, true) == 1000000 + 1500000);
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, false) == 800000 + 1200000);

        // Verify initial positions
        verify_positions(bulk_maker_addr, taker_addr, 0, 0);
        verify_positions(single_maker_addr, taker_addr, 0, 0);

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, bulk_maker = @0x456, single_maker = @0x789, taker = @0xabc
    )]
    public fun test_taker_matches_single_order_with_bulk_present(
        admin: &signer,
        market_signer: &signer,
        bulk_maker: &signer,
        single_maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let bulk_maker_addr = signer::address_of(bulk_maker);
        let single_maker_addr = signer::address_of(single_maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        // Place bulk orders
        let bid_prices = vector[1000, 990];
        let bid_sizes = vector[1000000, 1500000];
        let ask_prices = vector[1010, 1020];
        let ask_sizes = vector[800000, 1200000];

        let bulk_order_id_opt = place_bulk_order(
            &mut market,
            bulk_maker,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        let _bulk_order_id = bulk_order_id_opt.destroy_some();

        // Place a single order
        let single_order_id = place_maker_order(
            &mut market,
            single_maker,
            1005,
            1000000,
            true, // is_bid = true (bid)
            &mut event_store,
            false // not bulk
        );

        // Place a taker order that matches the single order
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1005,
            500000, // Half of single order
            false, // is_bid = false (selling)
            good_till_cancelled(),
            vector[500000],
            vector[1005],
            single_maker_addr,
            vector[single_order_id],
            vector[option::none()],
            vector[1000000],
            vector[1000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        // Verify positions after match
        verify_positions(single_maker_addr, taker_addr, 500000, 500000);
        assert!(get_position_size(bulk_maker_addr) == 0);

        // Verify order cleanup
        verify_orders_cleanup(&market, single_order_id, taker_order_id, false, true);
        // Single order should still exist with reduced size
        assert!(clearinghouse_test::order_exists(single_order_id));
        assert!(market.get_remaining_size(single_order_id) == 500000);
        // Bulk order should still exist unchanged
        assert!(clearinghouse_test::bulk_order_exists(bulk_maker_addr));
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, true) == 1000000 + 1500000);
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, false) == 800000 + 1200000);

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, bulk_maker = @0x456, single_maker = @0x789, taker = @0xabc
    )]
    public fun test_taker_matches_bulk_order_with_single_present(
        admin: &signer,
        market_signer: &signer,
        bulk_maker: &signer,
        single_maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let bulk_maker_addr = signer::address_of(bulk_maker);
        let single_maker_addr = signer::address_of(single_maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        // Place bulk orders
        let bid_prices = vector[1000, 990];
        let bid_sizes = vector[1000000, 1500000];
        let ask_prices = vector[1010, 1020];
        let ask_sizes = vector[800000, 1200000];

        let bulk_order_id_opt = place_bulk_order(
            &mut market,
            bulk_maker,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        let bulk_order_id = bulk_order_id_opt.destroy_some();

        // Place a single order
        let single_order_id = place_maker_order(
            &mut market,
            single_maker,
            1005,
            1000000,
            true, // is_bid = true (bid)
            &mut event_store,
            false // not bulk
        );

        // Place a taker order that matches the bulk order
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1010,
            600000, // Part of bulk order
            true, // is_bid = true (buying)
            good_till_cancelled(),
            vector[600000],
            vector[1010],
            bulk_maker_addr,
            vector[bulk_order_id],
            vector[option::none()],
            vector[800000 + 1200000], // Original bulk order size
            vector[800000 + 1200000], // Original bulk order size
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        // Verify positions after match
        verify_positions(bulk_maker_addr, taker_addr, 600000, 600000);
        assert!(get_position_size(single_maker_addr) == 0);

        // Verify order cleanup
        verify_orders_cleanup(&market, bulk_order_id, taker_order_id, false, true);
        // Bulk order should still exist with reduced size
        assert!(clearinghouse_test::bulk_order_exists(bulk_maker_addr));
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, true) == 1000000 + 1500000);
        assert!(market.get_bulk_order_remaining_size(bulk_maker_addr, false) == 800000 + 1200000 - 600000);
        // Single order should still exist unchanged
        assert!(clearinghouse_test::order_exists(single_order_id));
        assert!(market.get_remaining_size(single_order_id) == 1000000);

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, bulk_maker = @0x456, single_maker = @0x789, taker = @0xabc
    )]
    public fun test_bulk_order_cancellation_with_single_order_remaining(
        admin: &signer,
        market_signer: &signer,
        bulk_maker: &signer,
        single_maker: &signer,
        taker: &signer
    ) {
        let market = setup_market(admin, market_signer);
        let bulk_maker_addr = signer::address_of(bulk_maker);
        let single_maker_addr = signer::address_of(single_maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        // Place bulk orders
        let bid_prices = vector[1000, 990];
        let bid_sizes = vector[1000000, 1500000];
        let ask_prices = vector[1010, 1020];
        let ask_sizes = vector[800000, 1200000];

        let bulk_order_id_opt = place_bulk_order(
            &mut market,
            bulk_maker,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        let _bulk_order_id = bulk_order_id_opt.destroy_some();

        // Place a single order
        let single_order_id = place_maker_order(
            &mut market,
            single_maker,
            1005,
            1000000,
            true, // is_bid = true (bid)
            &mut event_store,
            false // not bulk
        );

        // Cancel bulk order
        market.cancel_bulk_order(bulk_maker, &test_market_callbacks());

        // Verify bulk order is cancelled
        verify_bulk_order_cleanup(bulk_maker_addr);

        // Verify single order still exists
        assert!(clearinghouse_test::order_exists(single_order_id));
        assert!(market.get_remaining_size(single_order_id) == 1000000);

        // Place a taker order that should only match the single order
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1005,
            500000,
            false, // is_bid = false (selling)
            good_till_cancelled(),
            vector[500000],
            vector[1005],
            single_maker_addr,
            vector[single_order_id],
            vector[option::none()],
            vector[1000000],
            vector[1000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        // Verify positions after matches
        verify_positions(single_maker_addr, taker_addr, 500000, 500000);
        assert!(get_position_size(bulk_maker_addr) == 0);

        // Verify order cleanup
        verify_orders_cleanup(&market, single_order_id, taker_order_id, false, true);
        // Single order should still exist
        assert!(clearinghouse_test::order_exists(single_order_id));
        assert!(market.get_remaining_size(single_order_id) == 500000);

        market.destroy_market()
    }
}
