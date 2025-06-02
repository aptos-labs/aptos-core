#[test_only]
module aptos_experimental::market_tests {
    use std::option;
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
        place_maker_order_and_verify,
        place_taker_order_and_verify_fill,
        place_taker_order,
        verify_cancel_event,
        verify_fills
    };
    use aptos_experimental::event_utils;
    use aptos_experimental::market::{
        good_till_cancelled,
        post_only,
        immediate_or_cancel,
        new_market,
        new_market_config
    };

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_gtc(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();
        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
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

        place_taker_order_and_verify_fill(
            &mut market,
            taker,
            1000,
            1000000,
            false,
            good_till_cancelled(),
            vector[1000000],
            vector[1000],
            maker_addr,
            vector[maker_order_id],
            vector[2000000],
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(),
            &test_market_callbacks()
        );
        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);
        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_post_only_success(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();

        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
                1000000,
                true,
                post_only(),
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);

        // Ensure the post only order was posted to the order book
        assert!(
            market.get_remaining_size(signer::address_of(maker), maker_order_id)
                == 1000000
        );
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
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();

        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let _maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
                1000000, // 1 BTC
                true, // is_buy
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order which is marked as post only but will immediately match - this should fail
        let taker_order_id =
            place_maker_order_and_verify(
                &mut market,
                taker,
                1000,
                1000000, // 1 BTC
                false, // is_buy
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
        assert!(
            market.get_remaining_size(signer::address_of(taker), taker_order_id) == 0
        );
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
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
                1000000, // 1 BTC
                true, // is_buy
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order will be immediately match in the same transaction
        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                1000,
                1000000, // 1 BTC
                false, // is_buy
                immediate_or_cancel(), // order_type
                vector[1000000],
                vector[1000],
                maker_addr,
                vector[maker_order_id],
                vector[1000000],
                &mut event_store,
                false,
                option::none(),
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);

        // Ensure the post only order was posted to the order book
        assert!(
            market.get_remaining_size(signer::address_of(taker), taker_order_id) == 0
        );
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
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
                1000000, // 1 BTC
                true, // is_buy
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order is IOC, which will partially match and remaining will be cancelled
        let taker_order_id =
            place_taker_order_and_verify_fill(
                &mut market,
                taker,
                1000,
                2000000, // 2 BTC
                false, // is_buy
                immediate_or_cancel(), // order_type
                vector[1000000],
                vector[1000],
                maker_addr,
                vector[maker_order_id],
                vector[1000000],
                &mut event_store,
                true,
                option::none(),
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Make sure no matches triggered by post only order
        assert!(get_position_size(maker_addr) == 1000000);
        assert!(get_position_size(taker_addr) == 1000000);

        // Ensure the post only order was posted to the order book
        assert!(
            market.get_remaining_size(signer::address_of(taker), taker_order_id) == 0
        );
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
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let _maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
                1000000, // 1 BTC
                true, // is_buy
                good_till_cancelled(), // order_type
                &mut event_store,
                false,
                false,
                new_test_order_metadata(),
                &test_market_callbacks()
            );

        // Taker order is IOC, which will not be matched and should be cancelled
        let taker_order_id =
            place_maker_order_and_verify(
                &mut market,
                taker,
                1200,
                1000000, // 1 BTC
                false, // is_buy
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

        assert!(
            market.get_remaining_size(signer::address_of(taker), taker_order_id) == 0
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
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        // Place maker order
        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000, // price
                500000, // 0.5 BTC
                true, // is_buy
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
                1000,
                1000000, // 1 BTC
                false, // is_buy
                good_till_cancelled(),
                vector[500000], // 0.5 BTC
                vector[1000],
                maker_addr,
                vector[maker_order_id],
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
        assert!(market.get_remaining_size(maker_addr, maker_order_id) == 0);

        // Taker order partially filled
        assert!(
            market.get_remaining_size(taker_addr, taker_order_id) == 500000 // 0.5 BTC remaining
        );

        market.destroy_market()
    }

    #[test(
        admin = @0x1, market_signer = @0x123, maker = @0x456, taker = @0x789
    )]
    public fun test_order_multiple_fills(
        admin: &signer,
        market_signer: &signer,
        maker: &signer,
        taker: &signer
    ) {
        // Setup accounts
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let event_store = event_utils::new_event_store();
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        // Place several maker order with small sizes.
        let i = 1;
        let maker_order_ids = vector::empty<u64>();
        let expected_fill_sizes = vector::empty<u64>();
        let fill_prices = vector::empty<u64>();
        let maker_orig_sizes = vector::empty<u64>();
        while (i < 6) {
            let maker_order_id =
                place_maker_order_and_verify(
                    &mut market,
                    maker,
                    1000 - i,
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
            expected_fill_sizes.push_back(10000 * i);
            maker_orig_sizes.push_back(10000 * i);
            fill_prices.push_back(1000 - i);
            i += 1;
        };
        let total_fill_size = expected_fill_sizes.fold(0, |acc, x| acc + x);

        // Order not matched yet, so the balance should not change
        assert!(get_position_size(maker_addr) == 0);
        assert!(get_position_size(taker_addr) == 0);
        place_taker_order_and_verify_fill(
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
            maker_orig_sizes,
            &mut event_store,
            false,
            option::none(),
            new_test_order_metadata(),
            &test_market_callbacks()
        );
        assert!(get_position_size(maker_addr) == total_fill_size);
        assert!(get_position_size(taker_addr) == total_fill_size);
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
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let maker_addr = signer::address_of(maker);
        let taker_addr = signer::address_of(taker);

        let event_store = event_utils::new_event_store();
        let maker_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker,
                1000,
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

        place_taker_order_and_verify_fill(
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
            vector[2000000],
            &mut event_store,
            true,
            option::none(),
            new_test_order_metadata(),
            &test_market_callbacks_with_taker_cancelled()
        );
        // Make sure the maker order is reinserted
        assert!(market.get_remaining_size(maker_addr, maker_order_id) == 1500000);
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
        let market = new_market(admin, market_signer, new_market_config(false));
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let maker2_addr = signer::address_of(maker2);
        let event_store = event_utils::new_event_store();
        let maker1_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker1,
                1001,
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
            place_maker_order_and_verify(
                &mut market,
                maker2,
                1000,
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
                1000,
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
            1000,
            1000000,
            false,
            vector[1000000],
            vector[1000],
            maker2_addr,
            vector[maker2_order_id],
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
        let market = new_market(admin, market_signer, new_market_config(true));
        clearinghouse_test::initialize(admin);
        let maker1_addr = signer::address_of(maker1);
        let event_store = event_utils::new_event_store();
        let maker1_order_id =
            place_maker_order_and_verify(
                &mut market,
                maker1,
                1001,
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
            place_maker_order_and_verify(
                &mut market,
                maker2,
                1000,
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
                1000,
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
            1001,
            1000000,
            false,
            vector[1000000],
            vector[1001],
            maker1_addr,
            vector[maker1_order_id],
            vector[2000000],
            &mut event_store,
            false
        );
        market.destroy_market()
    }
}
