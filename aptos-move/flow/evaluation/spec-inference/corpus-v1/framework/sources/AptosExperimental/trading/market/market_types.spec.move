spec aptos_experimental::market_types {

    spec cleanup_bulk_order_at_price<M: copy + drop + store, R: copy + drop + store>(
        self: &0x7::market_types::MarketClearinghouseCallbacks<M, R>,
        account: address,
        order_id: 0x5::order_book_types::OrderId,
        is_bid: bool,
        price: u64,
        cleanup_size: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] ensures_of<self.cleanup_bulk_order_at_price_f>(
            account, order_id, is_bid, price, cleanup_size
        );
        aborts_if aborts_of<self.cleanup_bulk_order_at_price_f>(
            account, order_id, is_bid, price, cleanup_size
        );
    }

    spec emit_event_for_bulk_order_cancelled<M: copy + drop + store>(
        self: &0x7::market_types::Market<M>,
        order_id: 0x5::order_book_types::OrderId,
        sequence_number: u64,
        user: address,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        cancellation_reason: 0x1::option::Option<0x7::market_types::OrderCancellationReason>
    ) {
        use 0x1::event;
        use 0x5::order_book_types;
        pragma opaque = true;
        ensures [inferred] self.config.allow_events_emission ==>
            ensures_of<event::emit<BulkOrderModifiedEvent>> (
                BulkOrderModifiedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_book_types::get_order_id_value(order_id),
                    sequence_number: sequence_number,
                    user: user,
                    bid_prices: vec<u64>(),
                    bid_sizes: vec<u64>(),
                    ask_prices: vec<u64>(),
                    ask_sizes: vec<u64>(),
                    cancelled_bid_prices: cancelled_bid_prices,
                    cancelled_bid_sizes: cancelled_bid_sizes,
                    cancelled_ask_prices: cancelled_ask_prices,
                    cancelled_ask_sizes: cancelled_ask_sizes,
                    previous_seq_num: sequence_number,
                    cancellation_reason: cancellation_reason
                }
            );
        aborts_if [inferred] self.config.allow_events_emission
            && aborts_of<event::emit<BulkOrderModifiedEvent>> (
                BulkOrderModifiedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_book_types::get_order_id_value(order_id),
                    sequence_number: sequence_number,
                    user: user,
                    bid_prices: vec<u64>(),
                    bid_sizes: vec<u64>(),
                    ask_prices: vec<u64>(),
                    ask_sizes: vec<u64>(),
                    cancelled_bid_prices: cancelled_bid_prices,
                    cancelled_bid_sizes: cancelled_bid_sizes,
                    cancelled_ask_prices: cancelled_ask_prices,
                    cancelled_ask_sizes: cancelled_ask_sizes,
                    previous_seq_num: sequence_number,
                    cancellation_reason: cancellation_reason
                }
            );
    }

    spec emit_event_for_bulk_order_placed<M: copy + drop + store>(
        self: &0x7::market_types::Market<M>,
        order_id: 0x5::order_book_types::OrderId,
        sequence_number: u64,
        user: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: u64
    ) {
        use 0x1::event;
        use 0x5::order_book_types;
        pragma opaque = true;
        ensures [inferred] self.config.allow_events_emission ==>
            ensures_of<event::emit<BulkOrderPlacedEvent>> (
                BulkOrderPlacedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_book_types::get_order_id_value(order_id),
                    sequence_number: sequence_number,
                    user: user,
                    bid_prices: bid_prices,
                    bid_sizes: bid_sizes,
                    ask_prices: ask_prices,
                    ask_sizes: ask_sizes,
                    cancelled_bid_prices: cancelled_bid_prices,
                    cancelled_bid_sizes: cancelled_bid_sizes,
                    cancelled_ask_prices: cancelled_ask_prices,
                    cancelled_ask_sizes: cancelled_ask_sizes,
                    previous_seq_num: previous_seq_num
                }
            );
        aborts_if [inferred] self.config.allow_events_emission
            && aborts_of<event::emit<BulkOrderPlacedEvent>> (
                BulkOrderPlacedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_book_types::get_order_id_value(order_id),
                    sequence_number: sequence_number,
                    user: user,
                    bid_prices: bid_prices,
                    bid_sizes: bid_sizes,
                    ask_prices: ask_prices,
                    ask_sizes: ask_sizes,
                    cancelled_bid_prices: cancelled_bid_prices,
                    cancelled_bid_sizes: cancelled_bid_sizes,
                    cancelled_ask_prices: cancelled_ask_prices,
                    cancelled_ask_sizes: cancelled_ask_sizes,
                    previous_seq_num: previous_seq_num
                }
            );
    }

    spec emit_event_for_bulk_order_rejection<M: copy + drop + store>(
        self: &0x7::market_types::Market<M>,
        user: address,
        sequence_number: u64,
        existing_sequence_number: u64
    ) {
        use 0x1::event;
        pragma opaque = true;
        ensures [inferred] self.config.allow_events_emission ==>
            ensures_of<event::emit<BulkOrderRejectionEvent>> (
                BulkOrderRejectionEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    user: user,
                    sequence_number: sequence_number,
                    existing_sequence_number: existing_sequence_number
                }
            );
        aborts_if [inferred] self.config.allow_events_emission
            && aborts_of<event::emit<BulkOrderRejectionEvent>> (
                BulkOrderRejectionEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    user: user,
                    sequence_number: sequence_number,
                    existing_sequence_number: existing_sequence_number
                }
            );
    }

    spec get_order_book_mut<M: copy + drop + store>(
        self: &mut 0x7::market_types::Market<M>
    ): &mut 0x7::order_book::OrderBook<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order_book;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec is_validation_result_valid(
        self: &0x7::market_types::ValidationResult
    ): bool {
        use 0x1::option;
        use 0x1::string;
        pragma opaque = true;
        ensures [inferred] result
            == option::is_none<string::String>(self.failure_reason);
        aborts_if [inferred] false;
    }

    spec place_bulk_order<M: copy + drop + store, R: copy + drop + store>(
        self: &0x7::market_types::MarketClearinghouseCallbacks<M, R>,
        account: address,
        order_id: 0x5::order_book_types::OrderId,
        bid_prices: &vector<u64>,
        bid_sizes: &vector<u64>,
        ask_prices: &vector<u64>,
        ask_sizes: &vector<u64>,
        cancelled_bid_prices: &vector<u64>,
        cancelled_bid_sizes: &vector<u64>,
        cancelled_ask_prices: &vector<u64>,
        cancelled_ask_sizes: &vector<u64>,
        metadata: &M
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] ensures_of<self.place_bulk_order_f>(
            account,
            order_id,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            metadata
        );
        aborts_if aborts_of<self.place_bulk_order_f>(
            account,
            order_id,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            metadata
        );
    }

    spec validate_bulk_order_placement<M: copy + drop + store, R: copy + drop + store>(
        self: &0x7::market_types::MarketClearinghouseCallbacks<M, R>,
        account: address,
        bids_prices: &vector<u64>,
        bids_sizes: &vector<u64>,
        asks_prices: &vector<u64>,
        asks_sizes: &vector<u64>,
        order_metadata: &M
    ): 0x7::market_types::ValidationResult {
        pragma opaque = true;
        // This is a direct function-value forwarding boundary.  Its normal and
        // abort behavior must remain coupled to the callback rather than
        // treating `result_of` as an untrusted inference carrier.
        ensures [inferred] result
            == result_of<self.validate_bulk_order_placement_f>(
                account,
                bids_prices,
                bids_sizes,
                asks_prices,
                asks_sizes,
                order_metadata
            );
        aborts_if [inferred] aborts_of<self.validate_bulk_order_placement_f>(
            account,
            bids_prices,
            bids_sizes,
            asks_prices,
            asks_sizes,
            order_metadata
        );
    }
}
