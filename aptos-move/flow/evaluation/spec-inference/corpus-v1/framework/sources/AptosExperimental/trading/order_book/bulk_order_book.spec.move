spec aptos_experimental::bulk_order_book {

    spec activate_first_price_level_for_side<M: copy + drop + store>(
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order: &0x5::bulk_order_types::BulkOrder<M>,
        order_id: 0x5::order_book_types::OrderId,
        is_bid: bool
    ) {
        use 0x1::option;
        use 0x5::order_book_types;
        use 0x5::bulk_order_types;
        use 0x7::price_time_index;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]!option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) ==>
            price_time_idx == old(price_time_idx);
        ensures [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) ==>
            ensures_of<price_time_index::place_maker_order>(
                price_time_idx,
                order_id,
                order_book_types::bulk_order_type(),
                option::destroy_some<u64>(
                    bulk_order_types::get_active_price<M>(
                        bulk_order_types::get_order_request<M>(order), is_bid
                    )
                ),
                bulk_order_types::get_unique_priority_idx<M>(order),
                option::destroy_some<u64>(
                    bulk_order_types::get_active_size<M>(
                        bulk_order_types::get_order_request<M>(order), is_bid
                    )
                ),
                is_bid,
                price_time_idx
            );
        aborts_if [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) && aborts_of<option::destroy_some<u64>> (
            bulk_order_types::get_active_size<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        );
        aborts_if [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) && aborts_of<option::destroy_some<u64>> (
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        );
        aborts_if [inferred] aborts_of<bulk_order_types::get_active_size<M>> (
            bulk_order_types::get_order_request<M>(order), is_bid
        );
        aborts_if [inferred] aborts_of<bulk_order_types::get_active_price<M>> (
            bulk_order_types::get_order_request<M>(order), is_bid
        );
        aborts_if option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) && aborts_of<price_time_index::place_maker_order>(
            price_time_idx,
            order_id,
            order_book_types::bulk_order_type(),
            option::destroy_some<u64>(
                bulk_order_types::get_active_price<M>(
                    bulk_order_types::get_order_request<M>(order), is_bid
                )
            ),
            bulk_order_types::get_unique_priority_idx<M>(order),
            option::destroy_some<u64>(
                bulk_order_types::get_active_size<M>(
                    bulk_order_types::get_order_request<M>(order), is_bid
                )
            ),
            is_bid
        );
    }

    spec activate_first_price_levels<M: copy + drop + store>(
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order: &0x5::bulk_order_types::BulkOrder<M>,
        order_id: 0x5::order_book_types::OrderId
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]..S1 |~(
            ensures_of<activate_first_price_level_for_side<M>> (
                old(price_time_idx), order, order_id, true
            )
        );
        ensures [inferred] S1.. |~(
            ensures_of<activate_first_price_level_for_side<M>> (
                price_time_idx, order, order_id, false, price_time_idx
            )
        );
        aborts_if aborts_of<activate_first_price_level_for_side<M>> (
            price_time_idx, order, order_id, true
        );
        aborts_if exists intermediate: 0x7::price_time_index::PriceTimeIndex:
            ensures_of<activate_first_price_level_for_side<M>> (
                price_time_idx, order, order_id, true, intermediate
            ) && aborts_of<activate_first_price_level_for_side<M>> (
                intermediate, order, order_id, false
            );
    }

    spec cancel_active_order_for_side<M: copy + drop + store>(
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order: &0x5::bulk_order_types::BulkOrder<M>,
        is_bid: bool
    ) {
        use 0x1::option;
        use 0x5::bulk_order_types;
        use 0x7::price_time_index;
        pragma opaque = true;
        ensures [inferred]!option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) ==>
            price_time_idx == old(price_time_idx);
        ensures [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) ==>
            ensures_of<price_time_index::cancel_active_order>(
                old(price_time_idx),
                option::destroy_some<u64>(
                    bulk_order_types::get_active_price<M>(
                        bulk_order_types::get_order_request<M>(order), is_bid
                    )
                ),
                bulk_order_types::get_unique_priority_idx<M>(order),
                is_bid,
                result_of<price_time_index::cancel_active_order>(
                    old(price_time_idx),
                    option::destroy_some<u64>(
                        bulk_order_types::get_active_price<M>(
                            bulk_order_types::get_order_request<M>(order), is_bid
                        )
                    ),
                    bulk_order_types::get_unique_priority_idx<M>(order),
                    is_bid
                ),
                price_time_idx
            );
        aborts_if [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) && aborts_of<option::destroy_some<u64>> (
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        );
        aborts_if [inferred] aborts_of<bulk_order_types::get_active_price<M>> (
            bulk_order_types::get_order_request<M>(order), is_bid
        );
        aborts_if [inferred] option::is_some<u64>(
            bulk_order_types::get_active_price<M>(
                bulk_order_types::get_order_request<M>(order), is_bid
            )
        ) && aborts_of<price_time_index::cancel_active_order>(
            price_time_idx,
            option::destroy_some<u64>(
                bulk_order_types::get_active_price<M>(
                    bulk_order_types::get_order_request<M>(order), is_bid
                )
            ),
            bulk_order_types::get_unique_priority_idx<M>(order),
            is_bid
        );
    }

    spec cancel_active_orders<M: copy + drop + store>(
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order: &0x5::bulk_order_types::BulkOrder<M>
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]..S1 |~(
            ensures_of<cancel_active_order_for_side<M>> (old(price_time_idx), order, true)
        );
        ensures [inferred] S1.. |~(
            ensures_of<cancel_active_order_for_side<M>> (
                price_time_idx, order, false, price_time_idx
            )
        );
        aborts_if aborts_of<cancel_active_order_for_side<M>> (price_time_idx, order, true);
        aborts_if exists intermediate: 0x7::price_time_index::PriceTimeIndex:
            ensures_of<cancel_active_order_for_side<M>> (
                price_time_idx, order, true, intermediate
            ) && aborts_of<cancel_active_order_for_side<M>> (intermediate, order, false);
    }

    spec cancel_bulk_order<M: copy + drop + store>(
        self: &mut 0x7::bulk_order_book::BulkOrderBook<M>,
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        account: address
    ): 0x5::bulk_order_types::BulkOrder<M> {
        use 0x1::big_ordered_map;
        use 0x5::bulk_order_types;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures result == old(big_ordered_map::spec_get(self.orders, account));
        ensures big_ordered_map::spec_contains_key(self.orders, account);
        ensures ensures_of<bulk_order_types::set_empty<M>> (
            result, big_ordered_map::spec_get(self.orders, account)
        );
        ensures self.orders
            == big_ordered_map::spec_set(
                big_ordered_map::spec_remove(old(self).orders, account),
                account,
                big_ordered_map::spec_get(self.orders, account)
            );
        ensures self.order_id_to_address == old(self).order_id_to_address;
        ensures ensures_of<cancel_active_orders<M>> (
            old(price_time_idx), result, price_time_idx
        );
        aborts_if !big_ordered_map::spec_contains_key(self.orders, account);
        aborts_if big_ordered_map::spec_contains_key(self.orders, account)
            && aborts_of<cancel_active_orders<M>> (
                price_time_idx, big_ordered_map::spec_get(self.orders, account)
            );
    }

    spec get_bulk_order<M: copy + drop + store>(
        self: &0x7::bulk_order_book::BulkOrderBook<M>, account: address
    ): 0x5::bulk_order_types::BulkOrder<M> {
        use 0x1::big_ordered_map;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures result == big_ordered_map::spec_get(self.orders, account);
        aborts_if !big_ordered_map::spec_contains_key(self.orders, account);
    }

    spec get_remaining_size<M: copy + drop + store>(
        self: &0x7::bulk_order_book::BulkOrderBook<M>, account: address, is_bid: bool
    ): u64 {
        use 0x1::big_ordered_map;
        use 0x5::bulk_order_types;
        pragma opaque = true;
        let order = big_ordered_map::spec_get(self.orders, account);
        ensures result
            == result_of<bulk_order_types::get_total_remaining_size<M>> (
                bulk_order_types::get_order_request<M>(order), is_bid
            );
        aborts_if !big_ordered_map::spec_contains_key(self.orders, account);
        aborts_if big_ordered_map::spec_contains_key(self.orders, account)
            && aborts_of<bulk_order_types::get_total_remaining_size<M>> (
                bulk_order_types::get_order_request<M>(order), is_bid
            );
    }

    spec place_bulk_order<M: copy + drop + store>(
        self: &mut 0x7::bulk_order_book::BulkOrderBook<M>,
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        order_req: 0x5::bulk_order_types::BulkOrderRequest<M>
    ): 0x5::bulk_order_types::BulkOrderPlaceResponse<M> {
        use 0x1::big_ordered_map;
        use 0x5::bulk_order_types;
        use 0x7::order_id_generation;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
            && order_req.order_sequence_number
                <= old(
                    big_ordered_map::spec_get(self.orders, order_req.account).order_request
                    .order_sequence_number
                ) ==>
            result
                == result_of<bulk_order_types::new_bulk_order_place_response_rejection<M>> (
                    order_req.account,
                    order_req.order_sequence_number,
                    old(
                        big_ordered_map::spec_get(self.orders, order_req.account).order_request
                        .order_sequence_number
                    )
                );
        ensures old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
            && order_req.order_sequence_number
                <= old(
                    big_ordered_map::spec_get(self.orders, order_req.account).order_request
                    .order_sequence_number
                ) ==>
            big_ordered_map::spec_get(self.orders, order_req.account)
                == old(big_ordered_map::spec_get(self.orders, order_req.account));
        ensures old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
            && order_req.order_sequence_number
                <= old(
                    big_ordered_map::spec_get(self.orders, order_req.account).order_request
                    .order_sequence_number
                ) ==>
            self.order_id_to_address == old(self).order_id_to_address;
        ensures old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
            && order_req.order_sequence_number
                <= old(
                    big_ordered_map::spec_get(self.orders, order_req.account).order_request
                    .order_sequence_number
                ) ==>
            price_time_idx == old(price_time_idx);
        ensures !(
            old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
                && order_req.order_sequence_number
                    <= old(
                        big_ordered_map::spec_get(self.orders, order_req.account).order_request
                        .order_sequence_number
                    )
        ) ==>
            result_of<bulk_order_types::is_success_response<M>> (result);
        ensures big_ordered_map::spec_contains_key(self.orders, order_req.account);
        ensures !(
            old(big_ordered_map::spec_contains_key(self.orders, order_req.account))
                && order_req.order_sequence_number
                    <= old(
                        big_ordered_map::spec_get(self.orders, order_req.account).order_request
                        .order_sequence_number
                    )
        ) ==>
            result.order == big_ordered_map::spec_get(self.orders, order_req.account);
        aborts_if big_ordered_map::spec_contains_key(self.orders, order_req.account)
            && order_req.order_sequence_number
                > big_ordered_map::spec_get(self.orders, order_req.account).order_request.order_sequence_number
            && aborts_of<cancel_active_orders<M>> (
                price_time_idx, big_ordered_map::spec_get(self.orders, order_req.account)
            );
        aborts_if !big_ordered_map::spec_contains_key(self.orders, order_req.account)
            && big_ordered_map::spec_aborts_add(
                self.order_id_to_address,
                result_of<order_id_generation::next_order_id>(),
                order_req.account
            );
    }

    spec cancel_bulk_order_at_price<M: copy + drop + store>(
        self: &mut 0x7::bulk_order_book::BulkOrderBook<M>,
        price_time_idx: &mut 0x7::price_time_index::PriceTimeIndex,
        account: address,
        price: u64,
        is_bid: bool
    ): (u64, 0x5::bulk_order_types::BulkOrder<M>) {
        pragma opaque = true, aborts_if_is_partial = true;
        // Corpus dependency abstraction: WP emitted no usable condition.
        ensures true;
    }
}
