#[test_only]
module velor_experimental::market_test_utils {
    use std::option;
    use std::option::Option;
    use std::signer;
    use velor_experimental::clearinghouse_test;
    use velor_experimental::event_utils::{latest_emitted_events, EventStore};
    use velor_experimental::market_types::{
        order_status_cancelled,
        order_status_filled,
        order_status_open,
        MarketClearinghouseCallbacks
    };
    use velor_experimental::order_book_types::OrderIdType;
    use velor_experimental::order_book_types::TimeInForce;

    use velor_experimental::market::{OrderEvent, Market, OrderMatchResult};

    const U64_MAX: u64 = 0xffffffffffffffff;

    public fun place_order_and_verify<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        price: Option<u64>,
        size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        event_store: &mut EventStore,
        is_taker: bool,
        is_cancelled: bool,
        metadata: M,
        client_order_id: Option<u64>,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderIdType {
        let user_addr = signer::address_of(user);
        let (limit_price, is_taker) = if (price.is_some()) {
            market.place_limit_order(
                user,
                price.destroy_some(),
                size,
                is_bid, // is_bid
                time_in_force, // order_type
                option::none(), // trigger_condition
                metadata,
                client_order_id,
                1000,
                true,
                callbacks
            );
            (price.destroy_some(), is_taker)
        } else {
            // Market order
            market.place_market_order(
                user,
                size,
                is_bid, // is_buy
                metadata,
                client_order_id, // client_order_id
                1000,
                true,
                callbacks
            );
            if (is_bid) {
                (U64_MAX, true) // Market buy order
            } else {
                (1, true) // Market sell order
            }
        };
        let events = latest_emitted_events<OrderEvent>(event_store, option::none());
        if (!is_cancelled) {
            assert!(events.length() == 1);
        } else {
            assert!(events.length() == 2);
        };
        let order_place_event = events[0];
        let order_id = order_place_event.get_order_id_from_event();
        order_place_event.verify_order_event(
            order_id,
            client_order_id, // client_order_id
            market.get_market(),
            user_addr,
            size,
            size,
            size,
            limit_price,
            is_bid,
            is_taker,
            order_status_open()
        );
        if (!is_cancelled) {
            // Maker order is opened
            assert!(clearinghouse_test::is_maker_order_called(order_id));
        } else {
            // Maker order is cancelled
            assert!(!clearinghouse_test::is_maker_order_called(order_id));
        };
        if (is_cancelled) {
            let order_cancel_event = events[1];
            order_cancel_event.verify_order_event(
                order_id,
                client_order_id,
                market.get_market(),
                user_addr,
                size,
                0, // Remaining size is always 0 when the order is cancelled
                size,
                limit_price,
                is_bid,
                is_taker,
                order_status_cancelled()
            )
        };
        order_id
    }

    public fun place_taker_order<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        client_order_id: Option<u64>,
        taker_price: Option<u64>,
        size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        event_store: &mut EventStore,
        max_matches: Option<u32>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): (OrderIdType, OrderMatchResult) {
        let taker_addr = signer::address_of(taker);
        let max_matches =
            if (max_matches.is_none()) { 1000 }
            else {
                max_matches.destroy_some()
            };
        // Taker order will be immediately match in the same transaction
        let result =
            if (taker_price.is_some()) {
                market.place_limit_order(
                    taker,
                    taker_price.destroy_some(),
                    size,
                    is_bid, // is_bid
                    time_in_force, // order_type
                    option::none(), // trigger_condition
                    metadata,
                    client_order_id,
                    max_matches,
                    true,
                    callbacks
                )
            } else {
                market.place_market_order(
                    taker,
                    size,
                    is_bid, // is_bid
                    metadata,
                    client_order_id,
                    max_matches,
                    true,
                    callbacks
                )
            };

        let events = latest_emitted_events<OrderEvent>(event_store, option::some(1));
        let order_place_event = events[0];
        let order_id = order_place_event.get_order_id_from_event();
        let limit_price = if (taker_price.is_some()) {
            taker_price.destroy_some()
        } else {
            if (is_bid) { U64_MAX } else { 1 }
        };
        // Taker order is opened
        order_place_event.verify_order_event(
            order_id,
            client_order_id,
            market.get_market(),
            taker_addr,
            size,
            size,
            size,
            limit_price,
            is_bid,
            true,
            order_status_open()
        );
        (order_id, result)
    }

    public fun place_taker_order_and_verify_fill<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        limit_price: u64,
        size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        fill_sizes: vector<u64>,
        fill_prices: vector<u64>,
        maker_addr: address,
        maker_order_ids: vector<OrderIdType>,
        maker_client_order_ids: vector<Option<u64>>,
        maker_orig_sizes: vector<u64>,
        maker_remaining_sizes: vector<u64>,
        event_store: &mut EventStore,
        is_cancelled: bool,
        max_matches: Option<u32>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): (OrderIdType, OrderMatchResult) {
        let (order_id, result) =
            place_taker_order(
                market,
                taker,
                option::none(), // client_order_id
                option::some(limit_price),
                size,
                is_bid,
                time_in_force,
                event_store,
                max_matches,
                metadata,
                callbacks
            );

        verify_fills(
            market,
            taker,
            order_id, // taker_order_id
            option::none(), // taker_client_order_id
            limit_price,
            size,
            is_bid,
            fill_sizes,
            fill_prices,
            maker_addr,
            maker_order_ids,
            maker_client_order_ids,
            maker_orig_sizes,
            maker_remaining_sizes,
            event_store,
            is_cancelled
        );

        (order_id, result)
    }

    public fun verify_cancel_event<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        is_taker: bool,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        is_bid: bool,
        event_store: &mut EventStore
    ) {
        let user_addr = signer::address_of(user);
        let events = latest_emitted_events<OrderEvent>(event_store, option::some(1));
        assert!(events.length() == 1);
        let order_cancel_event = events[0];
        order_cancel_event.verify_order_event(
            order_id,
            client_order_id,
            market.get_market(),
            user_addr,
            orig_size,
            remaining_size,
            size_delta,
            price, // price
            is_bid,
            is_taker,
            order_status_cancelled()
        );
    }

    public fun verify_fills<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        taker_order_id: OrderIdType,
        taker_client_order_id: Option<u64>,
        taker_price: u64,
        size: u64,
        is_bid: bool,
        fill_sizes: vector<u64>,
        fill_prices: vector<u64>,
        maker_addr: address,
        maker_order_ids: vector<OrderIdType>,
        maker_client_order_ids: vector<Option<u64>>,
        maker_orig_sizes: vector<u64>,
        maker_remaining_sizes: vector<u64>,
        event_store: &mut EventStore,
        is_cancelled: bool
    ) {
        let taker_addr = signer::address_of(taker);
        let total_fill_size = fill_sizes.fold(0, |acc, fill_size| acc + fill_size);
        let events = latest_emitted_events<OrderEvent>(event_store, option::none());
        assert!(fill_sizes.length() == maker_order_ids.length());
        assert!(fill_prices.length() == fill_sizes.length());
        assert!(maker_orig_sizes.length() == fill_sizes.length());
        assert!(size >= total_fill_size);
        let is_partial_fill = size > total_fill_size;
        let num_expected_events = 2 * fill_sizes.length();
        if (is_cancelled || is_partial_fill) {
            // Cancelling (from IOC) will add an extra cancel event
            // Partial fill will add an extra open event
            num_expected_events += 1;
        };
        assert!(events.length() == num_expected_events);

        let fill_index = 0;
        let taker_total_fill = 0;
        while (fill_index < fill_sizes.length()) {
            let fill_size = fill_sizes[fill_index];
            let fill_price = fill_prices[fill_index];
            let maker_orig_size = maker_orig_sizes[fill_index];
            let maker_remaining_size = maker_remaining_sizes[fill_index];
            taker_total_fill += fill_size;
            let maker_order_id = maker_order_ids[fill_index];
            let maker_client_order_id = maker_client_order_ids[fill_index];
            // Taker order is filled
            let taker_order_fill_event = events[2 * fill_index];
            taker_order_fill_event.verify_order_event(
                taker_order_id,
                taker_client_order_id,
                market.get_market(),
                taker_addr,
                size,
                size - taker_total_fill,
                fill_size,
                fill_price,
                is_bid,
                true,
                order_status_filled()
            );
            // Maker order is filled
            let maker_order_fill_event = events[1 + 2 * fill_index];
            maker_order_fill_event.verify_order_event(
                maker_order_id,
                maker_client_order_id,
                market.get_market(),
                maker_addr,
                maker_orig_size,
                maker_remaining_size - fill_size,
                fill_size,
                fill_price,
                !is_bid,
                false,
                order_status_filled()
            );
            fill_index += 1;
        };
        if (is_cancelled) {
            // Taker order is cancelled
            let order_cancel_event = events[num_expected_events - 1];
            order_cancel_event.verify_order_event(
                taker_order_id,
                taker_client_order_id,
                market.get_market(),
                taker_addr,
                size,
                0, // Remaining size is always 0 when the order is cancelled
                size - taker_total_fill,
                taker_price,
                is_bid,
                true,
                order_status_cancelled()
            )
        } else if (is_partial_fill) {
            // Maker order is opened
            let order_open_event = events[num_expected_events - 1];
            order_open_event.verify_order_event(
                taker_order_id,
                taker_client_order_id,
                market.get_market(),
                taker_addr,
                size,
                size - total_fill_size,
                size - total_fill_size,
                taker_price,
                is_bid,
                false,
                order_status_open()
            )
        };
    }
}
