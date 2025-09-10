#[test_only]
module aptos_experimental::market_tests_common {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_experimental::clearinghouse_test;
    use aptos_experimental::clearinghouse_test::{
        test_market_callbacks,
        new_test_order_metadata,
        get_position_size,
        test_market_callbacks_with_taker_cancelled
    };
    use aptos_experimental::market_test_utils::{
        place_order_and_verify,
        place_taker_order_and_verify_fill,
        place_taker_order,
        verify_cancel_event,
        verify_fills
    };
    use aptos_experimental::event_utils;
    use aptos_experimental::market::{new_market, new_market_config, Market, OrderMatchResult};
    use aptos_experimental::order_book_types::OrderIdType;
    use aptos_experimental::order_book_types::{good_till_cancelled, post_only, immediate_or_cancel};

    const PRE_CANCEL_WINDOW_MICROS: u64 = 1000000; // 1 second
    const U64_MAX: u64 = 0xFFFFFFFFFFFFFFFF;

    // Helper function to setup market and clearinghouse
    public fun setup_market_and_clearinghouse(
        admin: &signer,
        market_signer: &signer,
        allow_self_matching: bool
    ): Market<clearinghouse_test::TestOrderMetadata> {
        let market = new_market(
            admin,
            market_signer,
            new_market_config(allow_self_matching, true, PRE_CANCEL_WINDOW_MICROS)
        );
        clearinghouse_test::initialize(admin);
        market
    }

    // Helper function to setup market with default settings (no self matching)
    public fun setup_market(
        admin: &signer,
        market_signer: &signer
    ): Market<clearinghouse_test::TestOrderMetadata> {
        setup_market_and_clearinghouse(admin, market_signer, false)
    }

    // Helper function to place a maker order with common parameters
    public fun place_maker_order(
        market: &mut Market<clearinghouse_test::TestOrderMetadata>,
        maker: &signer,
        price: u64,
        size: u64,
        is_bid: bool,
        event_store: &mut event_utils::EventStore,
        is_bulk: bool,
    ): OrderIdType {
        if (is_bulk) {
            // Use bulk order placement for single order
            let bid_prices = if (is_bid) { vector[price] } else { vector::empty<u64>() };
            let bid_sizes = if (is_bid) { vector[size] } else { vector::empty<u64>() };
            let ask_prices = if (!is_bid) { vector[price] } else { vector::empty<u64>() };
            let ask_sizes = if (!is_bid) { vector[size] } else { vector::empty<u64>() };
            let order_id_opt = place_bulk_order(market, maker, bid_prices, bid_sizes, ask_prices, ask_sizes);
            assert!(order_id_opt.is_some(), 1);
            order_id_opt.destroy_some()
        } else {
            place_order_and_verify(
                market,
                maker,
                option::some(price),
                size,
                is_bid,
                good_till_cancelled(),
                event_store,
                false,
                false,
                new_test_order_metadata(1),
                option::none(),
                &test_market_callbacks()
            )
        }
    }

    public fun place_bulk_order(
        market: &mut Market<clearinghouse_test::TestOrderMetadata>,
        maker: &signer,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
    ): Option<OrderIdType> {
        market.place_bulk_order(
            signer::address_of(maker),
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            &test_market_callbacks()
        )
    }

    // Helper function to place a taker order that fully fills
    public fun place_taker_order_full_fill(
        market: &mut Market<clearinghouse_test::TestOrderMetadata>,
        taker: &signer,
        price: u64,
        size: u64,
        is_bid: bool,
        maker_addr: address,
        maker_order_id: OrderIdType,
        maker_orig_size: u64,
        event_store: &mut event_utils::EventStore
    ): (OrderIdType, OrderMatchResult) {
        place_taker_order_and_verify_fill(
            market,
            taker,
            price,
            size,
            is_bid,
            good_till_cancelled(),
            vector[size],
            vector[price],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[maker_orig_size],
            vector[maker_orig_size],
            event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        )
    }

    // Helper function to verify position sizes
    public fun verify_positions(
        maker_addr: address,
        taker_addr: address,
        expected_maker_size: u64,
        expected_taker_size: u64
    ) {
        assert!(get_position_size(maker_addr) == expected_maker_size);
        assert!(get_position_size(taker_addr) == expected_taker_size);
    }

    // Helper function to verify order cleanup
    public fun verify_orders_cleanup(
        market: &Market<clearinghouse_test::TestOrderMetadata>,
        maker_order_id: OrderIdType,
        taker_order_id: OrderIdType,
        expect_maker_cleaned: bool,
        expect_taker_cleaned: bool
    ) {
        if (expect_maker_cleaned) {
            assert!(!clearinghouse_test::order_exists(maker_order_id));
            assert!(market.get_remaining_size(maker_order_id) == 0);
        };
        if (expect_taker_cleaned) {
            assert!(!clearinghouse_test::order_exists(taker_order_id));
            assert!(market.get_remaining_size(taker_order_id) == 0);
        };
    }

    public fun verify_order_cleanup(
        market: &Market<clearinghouse_test::TestOrderMetadata>,
        order_id: OrderIdType,
    ) {
            assert!(!clearinghouse_test::order_exists(order_id));
            assert!(market.get_remaining_size(order_id) == 0);
    }

    public fun verify_bulk_order_cleanup(
        account: address,
    ) {
            assert!(!clearinghouse_test::bulk_order_exists(account));
    }

    // Additional common test helper functions to eliminate duplication

    public fun test_gtc_taker_fully_filled_internal(
        market: &mut Market<clearinghouse_test::TestOrderMetadata>,
        maker: &signer,
        taker: &signer,
        use_bulk: bool
    ) {
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        let maker_order_id =
            place_maker_order(market, maker, 1000, 2000000, true, &mut event_store, use_bulk);

        // Order not filled yet, so size is 0
        verify_positions(maker_addr, taker_addr, 0, 0);

        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            market,
            taker,
            1000,
            1000000,
            false,
            good_till_cancelled(),
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[2000000],
            vector[2000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );
        verify_positions(maker_addr, taker_addr, 1000000, 1000000);
        verify_orders_cleanup(market, maker_order_id, taker_order_id, false, true);

        let (taker_order_id2, _) = place_taker_order_and_verify_fill(
            market,
            taker,
            1000,
            1000000,
            false,
            good_till_cancelled(),
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[2000000],
            vector[1000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );
        verify_positions(maker_addr, taker_addr, 2000000, 2000000);
        verify_orders_cleanup(market, maker_order_id, taker_order_id2, true, true);
    }

    public fun test_gtc_taker_partially_filled_helper(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            1000000,
            true,
            &mut event_store,
            is_bulk,
        );

        let (_taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1000,
            2000000,
            false,
            good_till_cancelled(),
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[1000000],
            vector[1000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(2),
            &test_market_callbacks()
        );
        verify_positions(maker_addr, taker_addr, 1000000, 1000000);
        if (is_bulk) {
            verify_bulk_order_cleanup(maker_addr);
        } else {
            verify_order_cleanup(&market, maker_order_id);
        };
        market.destroy_market()
    }

    public fun test_post_only_success_helper(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);
        let event_store = event_utils::new_event_store();

        let maker_order_id = place_maker_order(
            &mut market,
            maker1,
            1000,
            1000000,
            true,
            &mut event_store,
            is_bulk
        );

        // Place a post only order that should not match with the maker order
        let maker2_order_id = place_order_and_verify(
            &mut market,
            maker2,
            option::some(1100),
            1000000,
            false, // is_bid
            post_only(), // order_type
            &mut event_store,
            false,
            false,
            new_test_order_metadata(1),
            option::none(),
            &test_market_callbacks()
        );

        // Make sure no matches triggered by post only order
        verify_positions(maker1_addr, maker2_addr, 0, 0);

        // Ensure both orders are active in the order book
        if (is_bulk) {
            assert!(clearinghouse_test::bulk_order_exists(maker1_addr));
        } else {
            assert!(clearinghouse_test::order_exists(maker_order_id));
        };
        assert!(clearinghouse_test::order_exists(maker2_order_id));
        market.destroy_market()
    }

    public fun test_post_only_failure_helper(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            1000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Taker order which is marked as post only but will immediately match - this should fail
        let taker_order_id = place_order_and_verify(
            &mut market,
            taker,
            option::some(1000),
            1000000,
            false, // is_bid
            post_only(), // order_type
            &mut event_store,
            true,
            true,
            new_test_order_metadata(1),
            option::none(),
            &test_market_callbacks()
        );

        // Make sure no matches triggered by post only order
        verify_positions(maker_addr, taker_addr, 0, 0);

        // Ensure the post only order was not posted in the order book
        assert!(market.get_remaining_size(taker_order_id) == 0);
        // Verify that the taker order is not active
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        // The maker order should still be active
        if (is_bulk) {
            assert!(clearinghouse_test::bulk_order_exists(maker_addr));
        } else {
            assert!(clearinghouse_test::order_exists(maker_order_id));
        };
        market.destroy_market()
    }

    public fun test_order_full_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            1000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Taker order will be immediately match in the same transaction
        let limit_price =
            if (is_market_order) {
                1 // Market order has no price, use max to ensure it matches
            } else {
                1000
            };
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            limit_price,
            1000000,
            false, // is_bid
            immediate_or_cancel(), // order_type
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[1000000],
            vector[1000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        verify_positions(maker_addr, taker_addr, 1000000, 1000000);
        verify_orders_cleanup(&market, maker_order_id, taker_order_id, true, true);
        market.destroy_market()
    }



    public fun test_order_partial_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            1000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Taker order is IOC, which will partially match and remaining will be cancelled
        let limit_price =
            if (is_market_order) {
                1 // Market order has no price, use minimum to ensure it matches
            } else {
                1000
            };
        let (taker_order_id, _) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            limit_price,
            2000000,
            false, // is_bid
            immediate_or_cancel(), // order_type
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[1000000],
            vector[1000000],
            &mut event_store,
            true,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        verify_positions(maker_addr, taker_addr, 1000000, 1000000);
        verify_orders_cleanup(&market, maker_order_id, taker_order_id, true, true);
        market.destroy_market()
    }

    public fun test_order_no_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            1000000, // 1 BTC
            true,
            &mut event_store,
            is_bulk,
        );

        // Taker order is IOC, which will not be matched and should be cancelled
        let limit_price =
            if (is_market_order) {
                option::none() // Market order has no price
            } else {
                option::some(1200) // Limit price for limit order
            };
        let taker_order_id = place_order_and_verify(
            &mut market,
            taker,
            limit_price,
            1000000, // 1 BTC
            false, // is_bid
            immediate_or_cancel(), // order_type
            &mut event_store,
            false, // Despite it being a "taker", this order will not cross
            true,
            new_test_order_metadata(1),
            option::none(),
            &test_market_callbacks()
        );

        // Make sure no matches triggered by post only order
        verify_positions(maker_addr, taker_addr, 0, 0);

        // Ensure the taker order was not posted in the order book and was cleaned up
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        // The maker order should still be active
        if (is_bulk) {
            assert!(clearinghouse_test::bulk_order_exists(maker_addr));
            // For bulk orders, we can't check remaining size using the order ID
        } else {
            assert!(clearinghouse_test::order_exists(maker_order_id));
            assert!(market.get_remaining_size(maker_order_id) == 1000000);
        };
        assert!(market.get_remaining_size(taker_order_id) == 0);
        market.destroy_market()
    }

    public fun test_taker_partial_cancelled_maker_reinserted_helper(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_bulk: bool
    ) {
        let market = setup_market(admin, market_signer);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);
        let event_store = event_utils::new_event_store();

        let maker_order_id = place_maker_order(
            &mut market,
            maker,
            1000,
            2000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Order not filled yet, so size is 0
        verify_positions(maker_addr, taker_addr, 0, 0);

        let (taker_order_id, result) = place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1000,
            1000000,
            false,
            good_till_cancelled(),
            vector[500000], // Half of the taker order is filled and half is cancelled
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[option::none()],
            vector[2000000],
            vector[2000000],
            &mut event_store,
            true,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks_with_taker_cancelled()
        );
        // Make sure the taker was cancelled
        assert!(result.get_remaining_size_from_result() == 0);
        assert!(result.get_cancel_reason().is_some());
        // Make sure the maker order is reinserted
        if (is_bulk) {
            assert!(market.get_bulk_order_remaining_size(maker_addr, true) == 1500000);
            assert!(clearinghouse_test::bulk_order_exists(maker_addr));
        } else {
            assert!(market.get_remaining_size(maker_order_id) == 1500000);
            assert!(clearinghouse_test::order_exists(maker_order_id));
        };
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        market.destroy_market()
    }

    public fun test_self_matching_not_allowed_helper(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer,
        is_bulk: bool
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
            is_bulk,
        );

        let maker2_order_id = place_maker_order(
            &mut market,
            maker2,
            1000,
            2000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker1_addr) == 0);

        // This should result in a self match order which should be cancelled and maker2 order should be filled
        let (taker_order_id, _) = place_taker_order(
            &mut market,
            maker1,
            option::none(),
            option::some(1000),
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

        verify_fills(
            &mut market,
            maker1,
            taker_order_id,
            option::none(),
            1000,
            1000000,
            false,
            vector[1000000],
            vector[1000],
            maker2_addr,
            vector[maker2_order_id],
            vector[option::none()],
            vector[2000000],
            vector[2000000],
            &mut event_store,
            false
        );

        verify_positions(maker1_addr, maker2_addr, 1000000, 1000000);
        market.destroy_market()
    }

    public fun test_self_matching_allowed_helper(
        aptos_framework: &signer,
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer,
        is_bulk: bool
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let market = setup_market_and_clearinghouse(admin, market_signer, true);
        let maker1_addr = signer::address_of(maker1);
        let event_store = event_utils::new_event_store();

        let maker1_order_id = place_maker_order(
            &mut market,
            maker1,
            1001,
            2000000,
            true,
            &mut event_store,
            is_bulk,
        );

        let _ = place_maker_order(
            &mut market,
            maker2,
            1000,
            2000000,
            true,
            &mut event_store,
            is_bulk,
        );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker1_addr) == 0);

        // This should result in a self match order which should be matched against self.
        let (taker_order_id, _) = place_taker_order(
            &mut market,
            maker1,
            option::some(1),
            option::some(1000),
            1000000,
            false,
            good_till_cancelled(),
            &mut event_store,
            option::none(),
            new_test_order_metadata(1),
            &test_market_callbacks()
        );

        verify_fills(
            &mut market,
            maker1,
            taker_order_id,
            option::some(1),
            1001,
            1000000,
            false,
            vector[1000000],
            vector[1001],
            maker1_addr,
            vector[maker1_order_id],
            vector[option::none()],
            vector[2000000],
            vector[2000000],
            &mut event_store,
            false
        );
        market.destroy_market()
    }
}
