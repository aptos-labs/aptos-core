#[test_only]
module aptos_experimental::market_tests {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::vector;
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
    use aptos_experimental::market_types::{
        good_till_cancelled,
        post_only,
        immediate_or_cancel
    };
    use aptos_experimental::market::{new_market, new_market_config};
    use aptos_experimental::order_book_types::OrderIdType;

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc_taker_fully_filled(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();
        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);

        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(1000),
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );
        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(!clearinghouse_test::order_exists(taker_order_id));

        let taker_order_id2 =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(1000),
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        assert!(get_position_size(maker_addr) == 2000000);
        assert!(get_position_size(taker_addr) == 2000000);
        // Both orders should be filled and cleaned up
        assert!(!clearinghouse_test::order_exists(maker_order_id));
        assert!(!clearinghouse_test::order_exists(taker_order_id2));
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();
        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                1000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(1000),
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );
        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);
        assert!(clearinghouse_test::order_exists(taker_order_id));
        assert!(!clearinghouse_test::order_exists(maker_order_id));
        market.destroy_market()
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);

        let event_store = event_utils::new_event_store();

        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker1,
                option::some(1000),
                1000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Place a post only order that should not match with the maker order
        let maker2_order_id =
            place_order_and_verify(
                &mut market,
                maker2,
                option::some(1100),
                1000000,
                false, // is_bid
                post_only(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker1_addr) == 0);
        assert!(get_position_size(maker2_addr) == 0);

        // Ensure the post only order was posted to the order book
        assert!(market.get_remaining_size(maker_order_id) == 1000000);
        assert!(market.get_remaining_size(maker2_order_id) == 1000000);

        // Verify that the maker order is still active
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(clearinghouse_test::order_exists(maker2_order_id));

        market.destroy_market()
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();

        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                1000000,
                true, // is_bid
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order which is marked as post only but will immediately match - this should fail
        let taker_order_id =
            place_order_and_verify(
                &mut market,
                taker,
                option::some(1000),
                1000000,
                false, // is_bid
                post_only(), // order_type
                &mut event_store,
                true,
                true,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);

        // Ensure the post only order was not posted in the order book
        assert!(market.get_remaining_size(taker_order_id) == 0);
        // Verify that the taker order is not active
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        // The maker order should still be active
        assert!(clearinghouse_test::order_exists(maker_order_id));
        market.destroy_market()
    }

    public fun test_order_full_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                1000000,
                true, // is_bid
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order will be immediately match in the same transaction
        let limit_price =
            if (is_market_order) {
                option::none() // Market order has no price
            } else {
                option::some(1000) // Limit price for limit order
            };
        let taker_order_id =
            place_taker_order_and_verify_fill(
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);

        // Both orders should be filled and cleaned up
        assert!(!clearinghouse_test::order_exists(maker_order_id));
        assert!(!clearinghouse_test::order_exists(taker_order_id));

        assert!(market.get_remaining_size(taker_order_id) == 0);
        assert!(market.get_remaining_size(maker_order_id) == 0);
        market.destroy_market()
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
        test_order_full_match(admin, market_signer, maker, taker, false);
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
            true // is_market_order
        );
    }

    public fun test_order_partial_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                1000000,
                true, // is_bid
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order is IOC, which will partially match and remaining will be cancelled
        let limit_price =
            if (is_market_order) {
                option::none() // Market order has no price
            } else {
                option::some(1000) // Limit price for limit order
            };
        let taker_order_id =
            place_taker_order_and_verify_fill(
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);

        // Ensure both orders are cleaned up
        assert!(!clearinghouse_test::order_exists(maker_order_id));
        assert!(!clearinghouse_test::order_exists(taker_order_id));

        assert!(market.get_remaining_size(taker_order_id) == 0);
        assert!(market.get_remaining_size(maker_order_id) == 0);
        market.destroy_market()
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
            false // is_market_order
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
            true // is_market_order
        );
    }

    public fun test_order_no_match(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer,
        is_market_order: bool
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                1000000, // 1 BTC
                true, // is_bid
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order is IOC, which will not be matched and should be cancelled
        let limit_price =
            if (is_market_order) {
                option::none() // Market order has no price
            } else {
                option::some(1200) // Limit price for limit order
            };
        let taker_order_id =
            place_order_and_verify(
                &mut market,
                taker,
                limit_price,
                1000000, // 1 BTC
                false, // is_bid
                immediate_or_cancel(), // order_type
                &mut event_store,
                false, // Despite it being a "taker", this order will not cross
                true,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);

        // Ensure the taker order was not posted in the order book and was cleaned up
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        // The maker order should still be active
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(market.get_remaining_size(maker_order_id) == 1000000);
        assert!(market.get_remaining_size(taker_order_id) == 0);
        market.destroy_market()
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
            false // is_market_order
        );
    }

    #[test(admin = @0x1, market_signer = @0x123, taker = @0x789)]
    public fun test_market_order_no_match(
        admin: &signer, market_signer: &signer, taker: &signer
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();

        let _taker_order_id =
            place_order_and_verify(
                &mut market,
                taker,
                option::none(),
                1000000, // 1 BTC
                false, // is_buy
                immediate_or_cancel(), // order_type
                &mut event_store,
                false, // Despite it being a "taker", this order will not cross
                true,
                new_test_order_metadata(),
                &test_market_callbacks()
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        // Place maker order
        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000), // price
                500000, // 0.5 BTC
                true, // is_bid
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order that will fully consume maker order but still have remaining size
        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(1000),
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Check positions after fill
        assert!(get_position_size(maker_addr) == 500000); // Long 0.5 BTC
        assert!(get_position_size(taker_addr) == 500000); // Short 0.5 BTC

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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
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
            let maker_order_id =
                place_order_and_verify(
                    &mut market,
                    maker,
                    option::some(1000 - i),
                    10000 * i,
                    true,
                    good_till_cancelled(),
                    &mut event_store,
                    false,
                    false,
                    new_test_order_metadata(),
                    &test_market_callbacks()
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
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);
        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(990),
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );
        assert!(get_position_size(maker_addr) == total_fill_size);
        assert!(get_position_size(taker_addr) == total_fill_size);
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();
        let maker_order_id =
            place_order_and_verify(
                &mut market,
                maker,
                option::some(1000),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);

        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                option::some(1000),
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
                new_test_order_metadata(),
                &test_market_callbacks_with_taker_cancelled()
            );
        // Make sure the maker order is reinserted
        assert!(market.get_remaining_size(maker_order_id) == 1500000);
        assert!(clearinghouse_test::order_exists(maker_order_id));
        assert!(!clearinghouse_test::order_exists(taker_order_id));
        market.destroy_market()
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
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);
        let event_store = event_utils::new_event_store();
        let maker1_order_id =
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        let maker2_order_id =
            place_order_and_verify(
                &mut market,
                maker2,
                option::some(1000),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker1_addr) == 0);

        // This should result in a self match order which should be cancelled and maker2 order should be filled
        let taker_order_id =
            place_taker_order(
                &mut market,
                maker1,
                option::none(),
                option::some(1000),
                1000000,
                false,
                good_till_cancelled(),
                &mut event_store,
                option::none(),
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        verify_cancel_event(
            &mut market,
            maker1,
            false,
            maker1_order_id,
            option::none(),
            option::some(1001),
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
            option::some(1000),
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

        assert!(get_position_size(maker1_addr) == 1000000);
        assert!(get_position_size(maker2_addr) == 1000000);
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_allowed(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(true, true)
        );
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let event_store = event_utils::new_event_store();
        let maker1_order_id =
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        let _ =
            place_order_and_verify(
                &mut market,
                maker2,
                option::some(1000),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Order not filled yet, so size is 0
        assert!(get_position_size(maker1_addr) == 0);

        // This should result in a self match order which should be matched against self.
        let taker_order_id =
            place_taker_order(
                &mut market,
                maker1,
                option::some(1),
                option::some(1000),
                1000000,
                false,
                good_till_cancelled(),
                &mut event_store,
                option::none(),
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        verify_fills(
            &mut market,
            maker1,
            taker_order_id,
            option::some(1),
            option::some(1001),
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

    #[test(
        admin = @0x1, market_signer = @0x123, maker1 = @0x456, maker2 = @0x789
    )]
    public fun test_self_matching_not_allowed_no_match(
        admin: &signer,
        market_signer: &signer,
        maker1: &signer,
        maker2: &signer
    ) {
        // Setup accounts
        let market = new_market(
            admin,
            market_signer,
            new_market_config(false, true)
        );
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);
        let event_store = event_utils::new_event_store();
        let maker1_order_id =
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
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        let _ =
            place_order_and_verify(
                &mut market,
                maker2,
                option::some(1000),
                2000000,
                true,
                good_till_cancelled(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
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
            new_test_order_metadata(),
            &test_market_callbacks()
        );

        verify_cancel_event(
            &mut market,
            maker1,
            false,
            maker1_order_id,
            option::none(),
            option::some(1001),
            2000000,
            0,
            2000000,
            true,
            &mut event_store
        );

        assert!(get_position_size(maker1_addr) == 0);
        assert!(get_position_size(maker2_addr) == 0);
        market.destroy_market()
    }
}
