spec aptos_trading::bulk_order_types {

    spec get_sequence_number<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.order_sequence_number;
        aborts_if [inferred] false;
    }

    spec destroy_bulk_order<M: copy + drop + store>(
        self: 0x5::bulk_order_types::BulkOrder<M>
    ): (
        0x5::bulk_order_types::BulkOrderRequest<M>,
        0x5::order_book_types::OrderId,
        0x5::order_book_types::IncreasingIdx,
        u64
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order_request;
        ensures [inferred] result_2 == self.order_id;
        ensures [inferred] result_3 == self.unique_priority_idx;
        ensures [inferred] result_4 == self.creation_time_micros;
        aborts_if [inferred] false;
    }

    spec destroy_bulk_order_place_response_rejection<M: copy + drop + store>(
        self: 0x5::bulk_order_types::BulkOrderPlaceResponse<M>
    ): (address, u64, u64) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.account;
        ensures [inferred] result_2 == self.sequence_number;
        ensures [inferred] result_3 == self.existing_sequence_number;
        aborts_if [inferred] self is Success_V1;
    }

    spec destroy_bulk_order_place_response_success<M: copy + drop + store>(
        self: 0x5::bulk_order_types::BulkOrderPlaceResponse<M>
    ): (
        0x5::bulk_order_types::BulkOrder<M>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        0x1::option::Option<u64>
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order;
        ensures [inferred] result_2 == self.cancelled_bid_prices;
        ensures [inferred] result_3 == self.cancelled_bid_sizes;
        ensures [inferred] result_4 == self.cancelled_ask_prices;
        ensures [inferred] result_5 == self.cancelled_ask_sizes;
        ensures [inferred] result_6 == self.previous_seq_num;
        aborts_if [inferred] self is Rejection_V1;
    }

    spec destroy_bulk_order_request<M: copy + drop + store>(
        self: 0x5::bulk_order_types::BulkOrderRequest<M>
    ): (
        address, u64, vector<u64>, vector<u64>, vector<u64>, vector<u64>, M
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.account;
        ensures [inferred] result_2 == self.order_sequence_number;
        ensures [inferred] result_3 == self.bid_prices;
        ensures [inferred] result_4 == self.bid_sizes;
        ensures [inferred] result_5 == self.ask_prices;
        ensures [inferred] result_6 == self.ask_sizes;
        ensures [inferred] result_7 == self.metadata;
        aborts_if [inferred] false;
    }

    spec get_account<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>
    ): address {
        pragma opaque = true;
        ensures [inferred] result == self.account;
        aborts_if [inferred] false;
    }

    spec get_active_price<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): 0x1::option::Option<u64> {
        use 0x1::option;
        pragma opaque = true;
        ensures [inferred] is_bid && len(self.bid_prices) == 0 ==>
            result == option::none<u64>();
        ensures [inferred] is_bid && len(self.bid_prices) != 0 ==>
            result == option::some<u64>(self.bid_prices[0]);
        ensures [inferred]!is_bid && len(self.ask_prices) == 0 ==>
            result == option::none<u64>();
        ensures [inferred]!is_bid && len(self.ask_prices) != 0 ==>
            result == option::some<u64>(self.ask_prices[0]);
        aborts_if [inferred] is_bid
            && (len(self.bid_prices) != 0
                && !in_range(self.bid_prices, 0));
        aborts_if [inferred]!is_bid
            && (len(self.ask_prices) != 0
                && !in_range(self.ask_prices, 0));
    }

    spec get_active_size<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): 0x1::option::Option<u64> {
        use 0x1::option;
        pragma opaque = true;
        ensures [inferred] is_bid && len(self.bid_sizes) == 0 ==>
            result == option::none<u64>();
        ensures [inferred] is_bid && len(self.bid_sizes) != 0 ==>
            result == option::some<u64>(self.bid_sizes[0]);
        ensures [inferred]!is_bid && len(self.ask_sizes) == 0 ==>
            result == option::none<u64>();
        ensures [inferred]!is_bid && len(self.ask_sizes) != 0 ==>
            result == option::some<u64>(self.ask_sizes[0]);
        aborts_if [inferred] is_bid
            && (len(self.bid_sizes) != 0
                && !in_range(self.bid_sizes, 0));
        aborts_if [inferred]!is_bid
            && (len(self.ask_sizes) != 0
                && !in_range(self.ask_sizes, 0));
    }

    spec get_all_prices<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): vector<u64> {
        pragma opaque = true;
        ensures [inferred] is_bid ==> result == self.bid_prices;
        ensures [inferred]!is_bid ==> result == self.ask_prices;
        aborts_if [inferred] false;
    }

    spec get_all_prices_mut<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): &mut vector<u64> {
        pragma opaque = true;
        ensures [inferred] is_bid ==> result == self.bid_prices;
        ensures [inferred]!is_bid ==> result == self.ask_prices;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec trim_prices_start<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool, new_start: u64
    ): vector<u64> {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures is_bid ==>
            result == old(self).bid_prices[0..new_start];
        ensures is_bid ==>
            self
                == update_field(
                    old(self),
                    bid_prices,
                    old(self).bid_prices[new_start..len(old(self).bid_prices)]
                );
        ensures !is_bid ==>
            result == old(self).ask_prices[0..new_start];
        ensures !is_bid ==>
            self
                == update_field(
                    old(self),
                    ask_prices,
                    old(self).ask_prices[new_start..len(old(self).ask_prices)]
                );
        aborts_if is_bid && new_start > len(self.bid_prices);
        aborts_if !is_bid && new_start > len(self.ask_prices);
    }

    spec get_all_sizes<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): vector<u64> {
        pragma opaque = true;
        ensures [inferred] is_bid ==> result == self.bid_sizes;
        ensures [inferred]!is_bid ==> result == self.ask_sizes;
        aborts_if [inferred] false;
    }

    spec get_all_sizes_mut<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): &mut vector<u64> {
        pragma opaque = true;
        ensures [inferred] is_bid ==> result == self.bid_sizes;
        ensures [inferred]!is_bid ==> result == self.ask_sizes;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec trim_sizes_start<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool, new_start: u64
    ): vector<u64> {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures is_bid ==>
            result == old(self).bid_sizes[0..new_start];
        ensures is_bid ==>
            self
                == update_field(
                    old(self),
                    bid_sizes,
                    old(self).bid_sizes[new_start..len(old(self).bid_sizes)]
                );
        ensures !is_bid ==>
            result == old(self).ask_sizes[0..new_start];
        ensures !is_bid ==>
            self
                == update_field(
                    old(self),
                    ask_sizes,
                    old(self).ask_sizes[new_start..len(old(self).ask_sizes)]
                );
        aborts_if is_bid && new_start > len(self.bid_sizes);
        aborts_if !is_bid && new_start > len(self.ask_sizes);
    }

    spec get_creation_time_micros<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrder<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.creation_time_micros;
        aborts_if [inferred] false;
    }

    spec get_order_id<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrder<M>
    ): 0x5::order_book_types::OrderId {
        pragma opaque = true;
        ensures [inferred] result == self.order_id;
        aborts_if [inferred] false;
    }

    spec get_order_request<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrder<M>
    ): &0x5::bulk_order_types::BulkOrderRequest<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order_request;
        aborts_if [inferred] false;
    }

    spec get_order_request_mut<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrder<M>
    ): &mut 0x5::bulk_order_types::BulkOrderRequest<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order_request;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec get_prices_and_sizes_mut<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): (&mut vector<u64>, &mut vector<u64>) {
        pragma opaque = true;
        ensures [inferred] is_bid ==> result_1 == self.bid_prices;
        ensures [inferred] is_bid ==> result_2 == self.bid_sizes;
        ensures [inferred]!is_bid ==> result_1 == self.ask_prices;
        ensures [inferred]!is_bid ==> result_2 == self.ask_sizes;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec get_unique_priority_idx<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrder<M>
    ): 0x5::order_book_types::IncreasingIdx {
        pragma opaque = true;
        ensures [inferred] result == self.unique_priority_idx;
        aborts_if [inferred] false;
    }

    spec is_rejection_response<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderPlaceResponse<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is Rejection_V1);
        aborts_if [inferred] false;
    }

    spec is_success_response<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderPlaceResponse<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is Success_V1);
        aborts_if [inferred] false;
    }

    spec new_bulk_order<M: copy + drop + store>(
        order_request: 0x5::bulk_order_types::BulkOrderRequest<M>,
        order_id: 0x5::order_book_types::OrderId,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        creation_time_micros: u64
    ): 0x5::bulk_order_types::BulkOrder<M> {
        pragma opaque = true;
        ensures [inferred] result
            == BulkOrder::V1<M> {
                order_request: order_request,
                order_id: order_id,
                unique_priority_idx: unique_priority_idx,
                creation_time_micros: creation_time_micros
            };
        aborts_if [inferred] false;
    }

    spec new_bulk_order_match<M: copy + drop + store>(
        order: &0x5::bulk_order_types::BulkOrder<M>, is_bid: bool, matched_size: u64
    ): 0x5::order_match_types::OrderMatch<M> {
        use 0x5::order_match_types;
        pragma opaque = true;
        ensures [inferred] is_bid ==>
            result
                == order_match_types::new_order_match<M>(
                    order_match_types::new_bulk_order_match_details<M>(
                        order.order_id,
                        order.order_request.account,
                        order.unique_priority_idx,
                        order.order_request.bid_prices[0],
                        order.order_request.bid_sizes[0] - matched_size,
                        true,
                        order.order_request.order_sequence_number,
                        order.creation_time_micros,
                        order.order_request.metadata
                    ),
                    matched_size
                );
        ensures [inferred]!is_bid ==>
            result
                == order_match_types::new_order_match<M>(
                    order_match_types::new_bulk_order_match_details<M>(
                        order.order_id,
                        order.order_request.account,
                        order.unique_priority_idx,
                        order.order_request.ask_prices[0],
                        order.order_request.ask_sizes[0] - matched_size,
                        false,
                        order.order_request.order_sequence_number,
                        order.creation_time_micros,
                        order.order_request.metadata
                    ),
                    matched_size
                );
        aborts_if [inferred] is_bid
            && order.order_request.bid_sizes[0] - matched_size < 0;
        aborts_if [inferred] is_bid && !in_range(order.order_request.bid_sizes, 0);
        aborts_if [inferred] is_bid && !in_range(order.order_request.bid_prices, 0);
        aborts_if [inferred]!is_bid
            && order.order_request.ask_sizes[0] - matched_size < 0;
        aborts_if [inferred]!is_bid && !in_range(order.order_request.ask_sizes, 0);
        aborts_if [inferred]!is_bid && !in_range(order.order_request.ask_prices, 0);
    }

    spec new_bulk_order_place_response_rejection<M: copy + drop + store>(
        account: address, sequence_number: u64, existing_sequence_number: u64
    ): 0x5::bulk_order_types::BulkOrderPlaceResponse<M> {
        pragma opaque = true;
        ensures [inferred] result
            == BulkOrderPlaceResponse::Rejection_V1<M> {
                account: account,
                sequence_number: sequence_number,
                existing_sequence_number: existing_sequence_number
            };
        aborts_if [inferred] false;
    }

    spec new_bulk_order_place_response_success<M: copy + drop + store>(
        order: 0x5::bulk_order_types::BulkOrder<M>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: 0x1::option::Option<u64>
    ): 0x5::bulk_order_types::BulkOrderPlaceResponse<M> {
        pragma opaque = true;
        ensures [inferred] result
            == BulkOrderPlaceResponse::Success_V1<M> {
                order: order,
                cancelled_bid_prices: cancelled_bid_prices,
                cancelled_bid_sizes: cancelled_bid_sizes,
                cancelled_ask_prices: cancelled_ask_prices,
                cancelled_ask_sizes: cancelled_ask_sizes,
                previous_seq_num: previous_seq_num
            };
        aborts_if [inferred] false;
    }

    spec new_bulk_order_request<M: copy + drop + store>(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M
    ): 0x5::bulk_order_types::BulkOrderRequest<M> {
        pragma opaque = true;
        ensures [inferred] result
            == BulkOrderRequest::V1<M> {
                account: account,
                order_sequence_number: sequence_number,
                bid_prices: bid_prices,
                bid_sizes: bid_sizes,
                ask_prices: ask_prices,
                ask_sizes: ask_sizes,
                metadata: metadata
            };
        aborts_if [inferred] false;
    }

    spec set_empty<M: copy + drop + store>(
        self: &mut 0x5::bulk_order_types::BulkOrder<M>
    ) {
        pragma opaque = true;
        ensures [inferred] self
            == update_field(
                old(self),
                order_request,
                update_field(
                    update_field(
                        old(self),
                        order_request,
                        update_field(
                            update_field(
                                old(self),
                                order_request,
                                update_field(
                                    update_field(
                                        old(self),
                                        order_request,
                                        update_field(
                                            old(self).order_request,
                                            bid_sizes,
                                            vec<u64>()
                                        )
                                    ).order_request,
                                    ask_sizes,
                                    vec<u64>()
                                )
                            ).order_request,
                            bid_prices,
                            vec<u64>()
                        )
                    ).order_request,
                    ask_prices,
                    vec<u64>()
                )
            );
        aborts_if [inferred] false;
    }

    spec get_total_remaining_size<M: copy + drop + store>(
        self: &0x5::bulk_order_types::BulkOrderRequest<M>, is_bid: bool
    ): u64 {
        pragma opaque = true;
        aborts_if is_bid && sum_sizes(self.bid_sizes, len(self.bid_sizes)) > MAX_U64;
        aborts_if !is_bid && sum_sizes(self.ask_sizes, len(self.ask_sizes)) > MAX_U64;
        ensures is_bid ==>
            result == sum_sizes(self.bid_sizes, len(self.bid_sizes));
        ensures !is_bid ==>
            result == sum_sizes(self.ask_sizes, len(self.ask_sizes));
    } proof {
        forall i: u64 { sum_sizes(self.bid_sizes, i) }
        apply sum_sizes_step_bound(self.bid_sizes, i, len(self.bid_sizes));
        forall i: u64 { sum_sizes(self.ask_sizes, i) }
        apply sum_sizes_step_bound(self.ask_sizes, i, len(self.ask_sizes));
    }

    /// Mathematical sum of the first `n` entries. Using `num` deliberately
    /// keeps the definition unbounded, so the contract can state the exact
    /// point at which the u64 accumulator aborts.
    spec fun sum_sizes(sizes: vector<u64>, n: num): num {
        if (n == 0) 0 else sum_sizes(sizes, n - 1) + sizes[n - 1]
    }

    /// Every non-negative vector element can only increase a prefix sum.
    /// This lifts a failing u64 addition to the full-sum abort condition.
    spec lemma sum_sizes_step_bound(sizes: vector<u64> , i: u64, n: u64) {
        requires i < n && n <= len(sizes);
        ensures sum_sizes(sizes, i) + sizes[i] <= sum_sizes(sizes, n);
    } proof {
        if (i + 1 < n) {
            apply sum_sizes_step_bound(sizes, i, n - 1);
        }
    }
}
