spec aptos_experimental::single_order_book {

    spec place_ready_maker_order_with_unique_idx<M: copy + drop + store>(
        self: &mut 0x7::single_order_book::SingleOrderBook<M>,
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order_req: 0x5::single_order_types::SingleOrderRequest<M>,
        ascending_idx: 0x5::order_book_types::IncreasingIdx
    ) {
        use 0x1::option;
        use 0x1::string;
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        use 0x5::single_order_types;
        use 0x7::price_time_index;
        pragma opaque = true;
        let order_id = single_order_types::get_order_id(order_req);
        let order_with_state = single_order_types::new_order_with_state(
            single_order_types::new_single_order(order_req, ascending_idx),
            true
        );
        let client_order_id = single_order_types::get_client_order_id(order_req);
        ensures [inferred]!option::is_some(client_order_id) ==>
            self
                == update_field(
                    old(self),
                    orders,
                    big_ordered_map::spec_set(
                        old(self).orders, order_id, order_with_state
                    )
                );
        ensures [inferred] option::is_some(client_order_id) ==>
            self
                == update_field(
                    update_field(
                        old(self),
                        orders,
                        big_ordered_map::spec_set(
                            old(self).orders, order_id, order_with_state
                        )
                    ),
                    client_order_ids,
                    big_ordered_map::spec_set(
                        old(self).client_order_ids,
                        order_book_types::new_account_client_order_id(
                            single_order_types::get_account(order_req),
                            option::destroy_some(client_order_id)
                        ),
                        order_id
                    )
                );
        ensures [inferred] ensures_of<price_time_index::place_maker_order>(
            old(price_time_idx),
            order_id,
            order_book_types::single_order_type(),
            single_order_types::get_price(order_req),
            ascending_idx,
            single_order_types::get_remaining_size(order_req),
            single_order_types::is_bid(order_req),
            price_time_idx
        );
        aborts_if [inferred] big_ordered_map::spec_contains_key(self.orders, order_id);
        aborts_if [inferred]!big_ordered_map::spec_contains_key(self.orders, order_id)
            && option::is_some(client_order_id)
            && big_ordered_map::spec_aborts_add(
                self.client_order_ids,
                order_book_types::new_account_client_order_id(
                    single_order_types::get_account(order_req),
                    option::destroy_some(client_order_id)
                ),
                order_id
            );
        aborts_if [inferred]!big_ordered_map::spec_contains_key(self.orders, order_id)
            && option::is_some(client_order_id)
            && aborts_of<option::destroy_some<string::String>> (client_order_id);
        aborts_if [inferred]!big_ordered_map::spec_contains_key(self.orders, order_id)
            && (
                !option::is_some(client_order_id)
                    || (
                        !big_ordered_map::spec_aborts_add(
                            self.client_order_ids,
                            order_book_types::new_account_client_order_id(
                                single_order_types::get_account(order_req),
                                option::destroy_some(client_order_id)
                            ),
                            order_id
                        )
                            && !aborts_of<option::destroy_some<string::String>> (
                                client_order_id
                            )
                    )
            )
            && aborts_of<price_time_index::place_maker_order>(
                price_time_idx,
                order_id,
                order_book_types::single_order_type(),
                single_order_types::get_price(order_req),
                ascending_idx,
                single_order_types::get_remaining_size(order_req),
                single_order_types::is_bid(order_req)
            );
    }

    /// AI-authored dependency contract. On success the selected order remains
    /// at the same key with only its remaining size decreased. An active order
    /// applies the corresponding update to the price-time index.
    spec decrease_order_size<M: copy + drop + store>(
        self: &mut 0x7::single_order_book::SingleOrderBook<M>,
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order_creator: address,
        order_id: 0x5::order_book_types::OrderId,
        size_delta: u64
    ) {
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        use 0x5::single_order_types;
        use 0x7::price_time_index;
        pragma opaque = true, aborts_if_is_partial = false;
        let previous = big_ordered_map::spec_get<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        );
        let updated = update_field(
            previous,
            order,
            update_field(
                previous.order,
                order_request,
                update_field(
                    previous.order.order_request,
                    remaining_size,
                    previous.order.order_request.remaining_size - size_delta
                )
            )
        );
        ensures big_ordered_map::spec_get<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        ) == updated;
        ensures self.client_order_ids == old(self).client_order_ids;
        ensures self.pending_orders == old(self).pending_orders;
        ensures !previous.is_active ==>
            price_time_idx == old(price_time_idx);
        ensures previous.is_active ==>
            ensures_of<price_time_index::decrease_order_size>(
                old(price_time_idx),
                previous.order.order_request.price,
                previous.order.unique_priority_idx,
                size_delta,
                previous.order.order_request.is_bid,
                price_time_idx
            );
        aborts_if !big_ordered_map::spec_contains_key<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        );
        aborts_if big_ordered_map::spec_contains_key<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        ) && big_ordered_map::spec_get<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        ).order.order_request.account != order_creator;
        aborts_if big_ordered_map::spec_contains_key<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        )
            && big_ordered_map::spec_get<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
                self.orders, order_id
            ).order.order_request.account == order_creator
            && big_ordered_map::spec_get<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
                self.orders, order_id
            ).order.order_request.remaining_size <= size_delta;
        aborts_if big_ordered_map::spec_contains_key<order_book_types::OrderId, single_order_types::OrderWithState<M>>(
            self.orders, order_id
        )
            && previous.order.order_request.account == order_creator
            && previous.order.order_request.remaining_size > size_delta
            && previous.is_active
            && aborts_of<price_time_index::decrease_order_size>(
                price_time_idx,
                previous.order.order_request.price,
                previous.order.unique_priority_idx,
                size_delta,
                previous.order.order_request.is_bid
            );
    }
}
