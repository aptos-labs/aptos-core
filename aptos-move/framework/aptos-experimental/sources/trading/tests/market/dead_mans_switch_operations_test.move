#[test_only]
module aptos_experimental::dead_mans_switch_operations_test {
    use std::option;
    use std::signer;
    use aptos_framework::timestamp;
    use aptos_framework::account;
    use aptos_experimental::clearinghouse_test;
    use aptos_experimental::market_types::{new_market, new_market_config, Market};
    use aptos_experimental::order_placement;
    use aptos_trading::order_book_types::{good_till_cancelled, new_order_id_type};
    use aptos_experimental::dead_mans_switch_operations;
    use aptos_experimental::market_bulk_order;

    const MIN_KEEP_ALIVE_TIME_SECS: u64 = 60; // 60 seconds minimum
    const KEEP_ALIVE_TIMEOUT_SECS: u64 = 300; // 5 minutes

    // Helper function to setup market with dead man's switch enabled
    fun setup_market_with_dms(
        admin: &signer, market_signer: &signer
    ): Market<clearinghouse_test::TestOrderMetadata> {
        timestamp::set_time_has_started_for_testing(admin);
        let market =
            new_market(
                admin,
                market_signer,
                new_market_config(
                    false, // allow_self_matching
                    true, // allow_events_emission
                    1, // pre_cancellation_window_secs
                    true, // enable_dead_mans_switch
                    MIN_KEEP_ALIVE_TIME_SECS
                )
            );
        clearinghouse_test::initialize(admin);
        market
    }

    // Helper to place an order and return order ID
    fun place_test_order(
        market: &mut Market<clearinghouse_test::TestOrderMetadata>,
        user: &signer,
        price: u64,
        size: u64,
        is_bid: bool
    ): aptos_trading::order_book_types::OrderId {
        let result =
            order_placement::place_limit_order(
                market,
                user,
                price,
                size,
                is_bid,
                good_till_cancelled(),
                option::none(), // trigger_condition
                clearinghouse_test::new_test_order_metadata(1),
                option::none(), // client_order_id
                100, // max_match_limit
                false, // cancel_on_match_limit
                &clearinghouse_test::test_market_callbacks()
            );
        result.get_order_id()
    }

    #[test]
    fun test_cleanup_expired_orders_after_timeout() {
        // Setup
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        let market = setup_market_with_dms(&admin, &mut market_signer);

        // Update keep-alive state for trader with 5 minute timeout
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Place several orders (bids at 100-101, asks at 200-201 to avoid matching)
        timestamp::fast_forward_seconds(10);
        let order_id_1 = place_test_order(&mut market, &trader, 100, 10, true);

        timestamp::fast_forward_seconds(5);
        let order_id_2 = place_test_order(&mut market, &trader, 101, 20, true);

        timestamp::fast_forward_seconds(5);
        let order_id_3 = place_test_order(&mut market, &trader, 200, 15, false);

        // Verify orders exist
        assert!(clearinghouse_test::order_exists(order_id_1));
        assert!(clearinghouse_test::order_exists(order_id_2));
        assert!(clearinghouse_test::order_exists(order_id_3));
        assert!(market.get_remaining_size(order_id_1) == 10);
        assert!(market.get_remaining_size(order_id_2) == 20);
        assert!(market.get_remaining_size(order_id_3) == 15);

        // Fast forward past the keep-alive timeout (5 minutes + buffer)
        timestamp::fast_forward_seconds(KEEP_ALIVE_TIMEOUT_SECS + 10);

        // Cleanup expired orders
        let order_ids = vector[order_id_1, order_id_2, order_id_3];
        dead_mans_switch_operations::cleanup_expired_orders(
            &mut market,
            order_ids,
            &clearinghouse_test::test_market_callbacks()
        );

        // Verify all orders are cancelled
        assert!(market.get_remaining_size(order_id_1) == 0);
        assert!(market.get_remaining_size(order_id_2) == 0);
        assert!(market.get_remaining_size(order_id_3) == 0);
        assert!(!clearinghouse_test::order_exists(order_id_1));
        assert!(!clearinghouse_test::order_exists(order_id_2));
        assert!(!clearinghouse_test::order_exists(order_id_3));

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_cleanup_does_not_cancel_valid_orders() {
        // Setup
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        let market = setup_market_with_dms(&admin, &mut market_signer);

        // Update keep-alive state for trader
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Place orders after keep-alive update
        timestamp::fast_forward_seconds(10);
        let order_id_1 = place_test_order(&mut market, &trader, 100, 10, true);
        let order_id_2 = place_test_order(&mut market, &trader, 101, 20, true);

        // Fast forward but stay within timeout
        timestamp::fast_forward_seconds(100); // Only 100 seconds, still valid

        // Renew keep-alive
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Try to cleanup - should not cancel anything
        let order_ids = vector[order_id_1, order_id_2];
        dead_mans_switch_operations::cleanup_expired_orders(
            &mut market,
            order_ids,
            &clearinghouse_test::test_market_callbacks()
        );

        // Verify orders still exist
        assert!(market.get_remaining_size(order_id_1) == 10);
        assert!(market.get_remaining_size(order_id_2) == 20);
        assert!(clearinghouse_test::order_exists(order_id_1));
        assert!(clearinghouse_test::order_exists(order_id_2));

        // Cleanup
        market.destroy_market();
    }

    #[test]
    #[
        expected_failure(
            abort_code = 0, location = aptos_experimental::dead_mans_switch_operations
        )
    ]
    fun test_cleanup_fails_when_dms_not_enabled() {
        // Setup market WITHOUT dead man's switch
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        timestamp::set_time_has_started_for_testing(&admin);
        let market =
            new_market(
                &admin,
                &mut market_signer,
                new_market_config(false, true, 1, false, 0) // DMS disabled
            );
        clearinghouse_test::initialize(&admin);

        // Try to cleanup - should abort with E_DEAD_MANS_SWITCH_NOT_ENABLED
        let order_ids = vector[new_order_id_type(1)];
        dead_mans_switch_operations::cleanup_expired_orders(
            &mut market,
            order_ids,
            &clearinghouse_test::test_market_callbacks()
        );

        market.destroy_market();
    }

    #[test]
    fun test_cleanup_expired_bulk_order() {
        // Setup
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Update keep-alive state for trader
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Place a bulk order
        timestamp::fast_forward_seconds(10);
        let bid_prices = vector[100, 99];
        let bid_sizes = vector[10, 20];
        let ask_prices = vector[200, 201];
        let ask_sizes = vector[15, 25];

        market_bulk_order::place_bulk_order(
            &mut market,
            trader_addr,
            1, // sequence number
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            clearinghouse_test::new_test_order_metadata(1),
            &clearinghouse_test::test_market_callbacks()
        );

        // Verify bulk order exists
        assert!(clearinghouse_test::bulk_order_exists(trader_addr));

        // Fast forward past the keep-alive timeout
        timestamp::fast_forward_seconds(KEEP_ALIVE_TIMEOUT_SECS + 10);

        // Cleanup expired bulk order
        dead_mans_switch_operations::cleanup_expired_bulk_order(
            &mut market,
            trader_addr,
            &clearinghouse_test::test_market_callbacks()
        );

        // Verify bulk order is cancelled
        assert!(!clearinghouse_test::bulk_order_exists(trader_addr));

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_cleanup_does_not_cancel_valid_bulk_order() {
        // Setup
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Update keep-alive state for trader
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Place a bulk order
        timestamp::fast_forward_seconds(10);
        let bid_prices = vector[100, 99];
        let bid_sizes = vector[10, 20];
        let ask_prices = vector[200, 201];
        let ask_sizes = vector[15, 25];

        market_bulk_order::place_bulk_order(
            &mut market,
            trader_addr,
            1, // sequence number
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            clearinghouse_test::new_test_order_metadata(1),
            &clearinghouse_test::test_market_callbacks()
        );

        // Fast forward but stay within timeout
        timestamp::fast_forward_seconds(100);

        // Renew keep-alive
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Try to cleanup - should not cancel
        dead_mans_switch_operations::cleanup_expired_bulk_order(
            &mut market,
            trader_addr,
            &clearinghouse_test::test_market_callbacks()
        );

        // Verify bulk order still exists
        assert!(clearinghouse_test::bulk_order_exists(trader_addr));

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_new_session_invalidates_old_orders() {
        // Setup
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Step 1: Update keep-alive state for trader
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Step 2: Place orders in the first session
        timestamp::fast_forward_seconds(10);
        let old_order_id_1 = place_test_order(&mut market, &trader, 100, 10, true);

        timestamp::fast_forward_seconds(5);
        let old_order_id_2 = place_test_order(&mut market, &trader, 101, 20, true);

        // Verify old orders exist
        assert!(clearinghouse_test::order_exists(old_order_id_1));
        assert!(clearinghouse_test::order_exists(old_order_id_2));
        assert!(market.get_remaining_size(old_order_id_1) == 10);
        assert!(market.get_remaining_size(old_order_id_2) == 20);

        // Step 3: Fast forward past the keep-alive timeout (session expires)
        timestamp::fast_forward_seconds(KEEP_ALIVE_TIMEOUT_SECS + 10);

        // Step 4: Start a new session with keep-alive update
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Step 5: Place new orders in the second session
        timestamp::fast_forward_seconds(10);
        let new_order_id_1 = place_test_order(&mut market, &trader, 102, 15, true);

        timestamp::fast_forward_seconds(5);
        let new_order_id_2 = place_test_order(&mut market, &trader, 200, 25, false);

        // Verify new orders exist
        assert!(clearinghouse_test::order_exists(new_order_id_1));
        assert!(clearinghouse_test::order_exists(new_order_id_2));
        assert!(market.get_remaining_size(new_order_id_1) == 15);
        assert!(market.get_remaining_size(new_order_id_2) == 25);

        // Step 6: Try to cleanup all orders (both old and new)
        let all_order_ids = vector[old_order_id_1, old_order_id_2, new_order_id_1, new_order_id_2];
        dead_mans_switch_operations::cleanup_expired_orders(
            &mut market,
            all_order_ids,
            &clearinghouse_test::test_market_callbacks()
        );

        // Step 7: Verify old orders from first session are cancelled
        assert!(!clearinghouse_test::order_exists(old_order_id_1));
        assert!(!clearinghouse_test::order_exists(old_order_id_2));
        assert!(market.get_remaining_size(old_order_id_1) == 0);
        assert!(market.get_remaining_size(old_order_id_2) == 0);

        // Step 8: Verify new orders from second session are still active
        assert!(clearinghouse_test::order_exists(new_order_id_1));
        assert!(clearinghouse_test::order_exists(new_order_id_2));
        assert!(market.get_remaining_size(new_order_id_1) == 15);
        assert!(market.get_remaining_size(new_order_id_2) == 25);

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_expired_maker_order_cancelled_on_match_attempt() {
        // This test verifies that when a taker tries to match against an expired maker order,
        // the maker order is automatically cancelled and the taker order is placed on the book

        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let maker = account::create_signer_for_test(@0x100);
        let taker = account::create_signer_for_test(@0x101);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Maker updates keep-alive state and places a bid order
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            signer::address_of(&maker),
            KEEP_ALIVE_TIMEOUT_SECS
        );

        timestamp::fast_forward_seconds(10);
        let maker_order_id = place_test_order(&mut market, &maker, 100, 50, true); // Bid @ 100

        // Verify maker order exists
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 50);

        // Fast forward past the keep-alive timeout to expire the maker's session
        // Note: We do NOT call cleanup_expired_orders
        timestamp::fast_forward_seconds(KEEP_ALIVE_TIMEOUT_SECS + 10);

        // Maker order should still be in the book (not cleaned up yet)
        assert!(clearinghouse_test::order_exists(maker_order_id));

        // Taker comes and tries to match with an ask order (sell @ 100)
        let taker_result =
            order_placement::place_limit_order(
                &mut market,
                &taker,
                100, // price
                50, // size
                false, // is_bid = false (ask/sell)
                good_till_cancelled(),
                option::none(),
                clearinghouse_test::new_test_order_metadata(2),
                option::none(),
                100,
                false,
                &clearinghouse_test::test_market_callbacks()
            );

        assert!(taker_result.total_fill_size() == 0); // No fill since maker order is expired

        // The expired maker order should be automatically cancelled during the match attempt
        assert!(!clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 0);

        // The taker order should be placed on the book instead of matching
        let taker_order_id = taker_result.get_order_id();
        assert!(clearinghouse_test::order_exists(taker_order_id));
        assert!(market.get_remaining_size(taker_order_id) == 50);

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_expired_taker_order_cancelled_before_matching() {
        // This test verifies that when a taker order itself is expired,
        // it gets cancelled before attempting to match

        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let maker = account::create_signer_for_test(@0x100);
        let taker = account::create_signer_for_test(@0x101);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Maker places an order (no dead man's switch for maker in this test)
        timestamp::fast_forward_seconds(10);
        let maker_order_id = place_test_order(&mut market, &maker, 100, 50, true); // Bid @ 100

        // Verify maker order exists
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 50);

        // Taker updates keep-alive state
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            signer::address_of(&taker),
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Fast forward past the taker's keep-alive timeout
        timestamp::fast_forward_seconds(KEEP_ALIVE_TIMEOUT_SECS + 10);

        // Taker tries to place an order that would normally match
        let taker_result =
            order_placement::place_limit_order(
                &mut market,
                &taker,
                100, // price
                50, // size
                false, // is_bid = false (ask/sell)
                good_till_cancelled(),
                option::none(),
                clearinghouse_test::new_test_order_metadata(2),
                option::none(),
                100,
                false,
                &clearinghouse_test::test_market_callbacks()
            );

        // The taker order should be cancelled due to expired session
        let cancel_reason = taker_result.get_cancel_reason();
        assert!(cancel_reason.is_some());
        assert!(
            order_placement::is_dead_mans_switch_expired(cancel_reason.destroy_some())
        );

        // Maker order should still exist (not affected)
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 50);

        // Cleanup
        market.destroy_market();
    }

    #[test]
    fun test_both_maker_and_taker_valid_orders_match_successfully() {
        // This test verifies that when both maker and taker have valid sessions,
        // the orders match successfully

        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let maker = account::create_signer_for_test(@0x100);
        let taker = account::create_signer_for_test(@0x101);

        let market = setup_market_with_dms(&admin, &market_signer);

        // Maker updates keep-alive state and places order
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            signer::address_of(&maker),
            KEEP_ALIVE_TIMEOUT_SECS
        );

        timestamp::fast_forward_seconds(10);
        let maker_order_id = place_test_order(&mut market, &maker, 100, 50, true); // Bid @ 100

        // Taker updates keep-alive state
        timestamp::fast_forward_seconds(10);
        dead_mans_switch_operations::keep_alive(
            &mut market,
            signer::address_of(&taker),
            KEEP_ALIVE_TIMEOUT_SECS
        );

        // Taker places matching order shortly after (within timeout)
        timestamp::fast_forward_seconds(10);
        let taker_result =
            order_placement::place_limit_order(
                &mut market,
                &taker,
                100, // price
                30, // size - partial fill
                false, // is_bid = false (ask/sell)
                good_till_cancelled(),
                option::none(),
                clearinghouse_test::new_test_order_metadata(2),
                option::none(),
                100,
                false,
                &clearinghouse_test::test_market_callbacks()
            );

        // Both orders should have valid sessions, so matching should succeed
        let cancel_reason = taker_result.get_cancel_reason();
        assert!(cancel_reason.is_none()); // No cancellation

        let taker_remaining = taker_result.get_remaining_size_from_result();
        assert!(taker_remaining == 0); // Taker fully filled

        // Maker order should be partially filled
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 20); // 50 - 30 = 20

        // Cleanup
        market.destroy_market();
    }

    #[test]
    #[
        expected_failure(
            abort_code = 0, location = aptos_experimental::dead_mans_switch_operations
        )
    ]
    fun test_keep_alive_fails_when_dms_not_enabled() {
        // Setup market WITHOUT dead man's switch
        let admin = account::create_signer_for_test(@0x1);
        let market_signer = account::create_signer_for_test(@0x2);
        let trader = account::create_signer_for_test(@0x100);
        let trader_addr = signer::address_of(&trader);

        timestamp::set_time_has_started_for_testing(&admin);
        let market: Market<clearinghouse_test::TestOrderMetadata> =
            new_market(
                &admin,
                &mut market_signer,
                new_market_config(false, true, 1, false, 0) // DMS disabled
            );
        clearinghouse_test::initialize(&admin);

        // Try to update keep-alive - should abort with E_DEAD_MANS_SWITCH_NOT_ENABLED
        dead_mans_switch_operations::keep_alive(
            &mut market,
            trader_addr,
            KEEP_ALIVE_TIMEOUT_SECS
        );

        market.destroy_market();
    }
}
