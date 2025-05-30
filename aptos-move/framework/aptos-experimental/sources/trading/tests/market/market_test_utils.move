#[test_only]
module aptos_experimental::market_test_utils {
    use std::option;
    use std::option::Option;
    use std::signer;
    use aptos_std::debug::print;
    use aptos_experimental::event_utils::{latest_emitted_events, EventStore};
    use aptos_experimental::market_types::MarketClearinghouseCallbacks;

    use aptos_experimental::market::{
        order_status_cancelled,
        order_status_filled,
        order_status_open,
        OrderEvent,
        Market
    };

    public fun place_maker_order_and_verify<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        price: u64,
        size: u64,
        is_buy: bool,
        time_in_force: u8,
        event_store: &mut EventStore,
        is_taker: bool,
        is_cancelled: bool,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): u64 {
        let user_addr = signer::address_of(user);
        market.place_order(
            user,
            price,
            size,
            is_buy, // is_buy
            time_in_force, // order_type
            option::none(), // trigger_condition
            metadata,
            1000,
            true,
            callbacks
        );
        let events = latest_emitted_events<OrderEvent>(event_store, option::none());
        print(&events.length());
        if (!is_cancelled) {
            assert!(events.length() == 1);
        } else {
            assert!(events.length() == 2);
        };
        let order_place_event = events[0];
        let order_id = order_place_event.get_order_id_from_event();
        order_place_event.verify_order_event(
            order_id,
            market.get_market(),
            user_addr,
            size,
            size,
            size,
            price,
            is_buy,
            is_taker,
            order_status_open()
        );
        if (is_cancelled) {
            let order_cancel_event = events[1];
            order_cancel_event.verify_order_event(
                order_id,
                market.get_market(),
                user_addr,
                size,
                0, // Remaining size is always 0 when the order is cancelled
                size,
                price,
                is_buy,
                is_taker,
                order_status_cancelled()
            )
        };
        order_id
    }

    public fun place_taker_order<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        taker_price: u64,
        size: u64,
        is_buy: bool,
        time_in_force: u8,
        event_store: &mut EventStore,
        max_fills: Option<u64>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): u64 {
        let taker_addr = signer::address_of(taker);
        let max_fills =
            if (max_fills.is_none()) { 1000 }
            else {
                max_fills.destroy_some()
            };
        // Taker order will be immediately match in the same transaction
        market.place_order(
            taker,
            taker_price,
            size,
            is_buy, // is_buy
            time_in_force, // order_type
            option::none(), // trigger_condition
            metadata,
            max_fills,
            true,
            callbacks
        );

        let events = latest_emitted_events<OrderEvent>(event_store, option::some(1));
        let order_place_event = events[0];
        let order_id = order_place_event.get_order_id_from_event();
        // Taker order is opened
        order_place_event.verify_order_event(
            order_id,
            market.get_market(),
            taker_addr,
            size,
            size,
            size,
            taker_price,
            is_buy,
            true,
            order_status_open()
        );
        order_id
    }

    public fun place_taker_order_and_verify_fill<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        taker_price: u64,
        size: u64,
        is_buy: bool,
        time_in_force: u8,
        fill_sizes: vector<u64>,
        fill_prices: vector<u64>,
        maker_addr: address,
        maker_order_ids: vector<u64>,
        maker_orig_sizes: vector<u64>,
        event_store: &mut EventStore,
        is_cancelled: bool,
        max_fills: Option<u64>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): u64 {
        let order_id =
            place_taker_order(
                market,
                taker,
                taker_price,
                size,
                is_buy,
                time_in_force,
                event_store,
                max_fills,
                metadata,
                callbacks
            );

        verify_fills(
            market,
            taker,
            order_id, // taker_order_id
            taker_price,
            size,
            is_buy,
            fill_sizes,
            fill_prices,
            maker_addr,
            maker_order_ids,
            maker_orig_sizes,
            event_store,
            is_cancelled
        );

        order_id
    }

    public fun verify_cancel_event<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        is_taker: bool,
        order_id: u64,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        is_buy: bool,
        event_store: &mut EventStore
    ) {
        let user_addr = signer::address_of(user);
        let events = latest_emitted_events<OrderEvent>(event_store, option::some(1));
        assert!(events.length() == 1);
        let order_cancel_event = events[0];
        order_cancel_event.verify_order_event(
            order_id,
            market.get_market(),
            user_addr,
            orig_size,
            remaining_size,
            size_delta,
            price, // price
            is_buy,
            is_taker,
            order_status_cancelled()
        );
    }

    public fun verify_fills<M: store + copy + drop>(
        market: &mut Market<M>,
        taker: &signer,
        taker_order_id: u64,
        taker_price: u64,
        size: u64,
        is_buy: bool,
        fill_sizes: vector<u64>,
        fill_prices: vector<u64>,
        maker_addr: address,
        maker_order_ids: vector<u64>,
        maker_orig_sizes: vector<u64>,
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
            taker_total_fill += fill_size;
            let maker_order_id = maker_order_ids[fill_index];
            // Taker order is filled
            let taker_order_fill_event = events[2 * fill_index];
            taker_order_fill_event.verify_order_event(
                taker_order_id,
                market.get_market(),
                taker_addr,
                size,
                size - taker_total_fill,
                fill_size,
                fill_price,
                is_buy,
                true,
                order_status_filled()
            );
            // Maker order is filled
            let maker_order_fill_event = events[1 + 2 * fill_index];
            maker_order_fill_event.verify_order_event(
                maker_order_id,
                market.get_market(),
                maker_addr,
                maker_orig_size,
                maker_orig_size - fill_size,
                fill_size,
                fill_price,
                !is_buy,
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
                market.get_market(),
                taker_addr,
                size,
                0, // Remaining size is always 0 when the order is cancelled
                size - taker_total_fill,
                taker_price,
                is_buy,
                true,
                order_status_cancelled()
            )
        } else if (is_partial_fill) {
            // Maker order is opened
            let order_open_event = events[num_expected_events - 1];
            order_open_event.verify_order_event(
                taker_order_id,
                market.get_market(),
                taker_addr,
                size,
                size - total_fill_size,
                size,
                taker_price,
                is_buy,
                false,
                order_status_open()
            )
        };
    }
}
