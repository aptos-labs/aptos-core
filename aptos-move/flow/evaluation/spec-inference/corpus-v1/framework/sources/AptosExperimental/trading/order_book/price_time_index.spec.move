spec aptos_experimental::price_time_index {

    spec best_ask_price(self: &0x7::price_time_index::PriceTimeIndex)
        : 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] big_ordered_map::spec_is_empty(self.sells) ==>
            result == option::none<u64>();
        ensures [inferred]!big_ordered_map::spec_is_empty(self.sells) ==>
            result
                == option::some<u64>(big_ordered_map::spec_key_at(self.sells, 0).price);
        aborts_if [inferred] false;
    }

    spec best_bid_price(self: &0x7::price_time_index::PriceTimeIndex)
        : 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] big_ordered_map::spec_is_empty(self.buys) ==>
            result == option::none<u64>();
        ensures [inferred]!big_ordered_map::spec_is_empty(self.buys) ==>
            result
                == option::some<u64>(
                    big_ordered_map::spec_key_at(
                        self.buys, big_ordered_map::spec_len(self.buys) - 1
                    ).price
                );
        aborts_if [inferred] false;
    }

    spec cancel_active_order(
        self: &mut 0x7::price_time_index::PriceTimeIndex,
        price: u64,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        is_bid: bool
    ): u64 {
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        pragma opaque = true;
        let cse_ = PriceAscTime { price: price, tie_breaker: unique_priority_idx };
        ensures [inferred] is_bid ==>
            result
                == big_ordered_map::spec_get<PriceDescTime, OrderData>(
                    old(self).buys,
                    PriceDescTime {
                        price: price,
                        tie_breaker: order_book_types::into_decreasing_idx_type(
                            unique_priority_idx
                        )
                    }
                ).size;
        ensures [inferred] is_bid ==>
            self
                == update_field(
                    old(self),
                    buys,
                    big_ordered_map::spec_remove<PriceDescTime, OrderData>(
                        old(self).buys,
                        PriceDescTime {
                            price: price,
                            tie_breaker: order_book_types::into_decreasing_idx_type(
                                unique_priority_idx
                            )
                        }
                    )
                );
        ensures [inferred]!is_bid ==>
            result
                == big_ordered_map::spec_get<PriceAscTime, OrderData>(
                    old(self).sells, cse_
                ).size;
        ensures [inferred]!is_bid ==>
            self
                == update_field(
                    old(self),
                    sells,
                    big_ordered_map::spec_remove<PriceAscTime, OrderData>(
                        old(self).sells, cse_
                    )
                );
        aborts_if [inferred] is_bid
            && big_ordered_map::spec_aborts_del<PriceDescTime, OrderData>(
                self.buys,
                PriceDescTime {
                    price: price,
                    tie_breaker: order_book_types::into_decreasing_idx_type(
                        unique_priority_idx
                    )
                }
            );
        aborts_if [inferred] is_bid
            && aborts_of<order_book_types::into_decreasing_idx_type>(unique_priority_idx);
        aborts_if [inferred]!is_bid
            && big_ordered_map::spec_aborts_del<PriceAscTime, OrderData>(
                self.sells, cse_
            );
    }

    spec increase_order_size(
        self: &mut 0x7::price_time_index::PriceTimeIndex,
        price: u64,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        size_delta: u64,
        is_bid: bool
    ) {
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        pragma opaque = true;
        let bid_key = PriceDescTime {
            price: price,
            tie_breaker: order_book_types::into_decreasing_idx_type(unique_priority_idx)
        };
        let ask_key = PriceAscTime { price: price, tie_breaker: unique_priority_idx };
        ensures [inferred] is_bid ==>
            self
                == update_field(
                    old(self),
                    buys,
                    big_ordered_map::spec_set(
                        old(self).buys,
                        bid_key,
                        update_field(
                            big_ordered_map::spec_get(old(self).buys, bid_key),
                            size,
                            big_ordered_map::spec_get(old(self).buys, bid_key).size
                                + size_delta
                        )
                    )
                );
        ensures [inferred]!is_bid ==>
            self
                == update_field(
                    old(self),
                    sells,
                    big_ordered_map::spec_set(
                        old(self).sells,
                        ask_key,
                        update_field(
                            big_ordered_map::spec_get(old(self).sells, ask_key),
                            size,
                            big_ordered_map::spec_get(old(self).sells, ask_key).size
                                + size_delta
                        )
                    )
                );
        aborts_if [inferred] is_bid
            && aborts_of<order_book_types::into_decreasing_idx_type>(unique_priority_idx);
        aborts_if [inferred] is_bid
            && !big_ordered_map::spec_contains_key(self.buys, bid_key);
        aborts_if [inferred] is_bid
            && big_ordered_map::spec_contains_key(self.buys, bid_key)
            && big_ordered_map::spec_get(self.buys, bid_key).size + size_delta
                > MAX_U64;
        aborts_if [inferred]!is_bid
            && !big_ordered_map::spec_contains_key(self.sells, ask_key);
        aborts_if [inferred]!is_bid
            && big_ordered_map::spec_contains_key(self.sells, ask_key)
            && big_ordered_map::spec_get(self.sells, ask_key).size + size_delta
                > MAX_U64;
    }

    /// AI-authored dependency contract. Decreasing an order changes exactly
    /// the selected price-time entry and preserves the opposite side.
    spec decrease_order_size(
        self: &mut 0x7::price_time_index::PriceTimeIndex,
        price: u64,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        size_delta: u64,
        is_bid: bool
    ) {
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        pragma opaque = true, aborts_if_is_partial = false;
        let bid_key = PriceDescTime {
            price: price,
            tie_breaker: order_book_types::into_decreasing_idx_type(unique_priority_idx)
        };
        let ask_key = PriceAscTime { price: price, tie_breaker: unique_priority_idx };
        ensures is_bid ==>
            self
                == update_field(
                    old(self),
                    buys,
                    big_ordered_map::spec_set<PriceDescTime, OrderData>(
                        old(self).buys,
                        bid_key,
                        update_field(
                            big_ordered_map::spec_get<PriceDescTime, OrderData>(
                                old(self).buys, bid_key
                            ),
                            size,
                            big_ordered_map::spec_get<PriceDescTime, OrderData>(
                                old(self).buys, bid_key
                            ).size - size_delta
                        )
                    )
                );
        ensures !is_bid ==>
            self
                == update_field(
                    old(self),
                    sells,
                    big_ordered_map::spec_set<PriceAscTime, OrderData>(
                        old(self).sells,
                        ask_key,
                        update_field(
                            big_ordered_map::spec_get<PriceAscTime, OrderData>(
                                old(self).sells, ask_key
                            ),
                            size,
                            big_ordered_map::spec_get<PriceAscTime, OrderData>(
                                old(self).sells, ask_key
                            ).size - size_delta
                        )
                    )
                );
        aborts_if is_bid
            && !big_ordered_map::spec_contains_key<PriceDescTime, OrderData>(
                self.buys, bid_key
            );
        aborts_if is_bid
            && big_ordered_map::spec_contains_key<PriceDescTime, OrderData>(
                self.buys, bid_key
            )
            && big_ordered_map::spec_get<PriceDescTime, OrderData>(self.buys, bid_key).size
                < size_delta;
        aborts_if !is_bid
            && !big_ordered_map::spec_contains_key<PriceAscTime, OrderData>(
                self.sells, ask_key
            );
        aborts_if !is_bid
            && big_ordered_map::spec_contains_key<PriceAscTime, OrderData>(
                self.sells, ask_key
            )
            && big_ordered_map::spec_get<PriceAscTime, OrderData>(self.sells, ask_key).size
                < size_delta;
    }

    spec is_taker_order(
        self: &0x7::price_time_index::PriceTimeIndex, price: u64, is_bid: bool
    ): bool {
        use 0x1::option;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] is_bid
            && option::is_some<u64>(result_of<best_ask_price>(self)) ==>
            result
                == (
                    price >= option::destroy_some<u64>(result_of<best_ask_price>(self))
                );
        ensures [inferred] is_bid
            && !option::is_some<u64>(result_of<best_ask_price>(self)) ==> result
            == false;
        ensures [inferred]!is_bid
            && option::is_some<u64>(result_of<best_bid_price>(self)) ==>
            result
                == (
                    price <= option::destroy_some<u64>(result_of<best_bid_price>(self))
                );
        ensures [inferred]!is_bid
            && !option::is_some<u64>(result_of<best_bid_price>(self)) ==> result
            == false;
        aborts_if [inferred] is_bid
            && (
                option::is_some<u64>(result_of<best_ask_price>(self))
                    && aborts_of<option::destroy_some<u64>> (
                        result_of<best_ask_price>(self)
                    )
            );
        aborts_if [inferred]!is_bid
            && (
                option::is_some<u64>(result_of<best_bid_price>(self))
                    && aborts_of<option::destroy_some<u64>> (
                        result_of<best_bid_price>(self)
                    )
            );
    }

    spec place_maker_order(
        self: &mut 0x7::price_time_index::PriceTimeIndex,
        order_id: 0x5::order_book_types::OrderId,
        order_book_type: 0x5::order_book_types::OrderType,
        price: u64,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        size: u64,
        is_bid: bool
    ) {
        use 0x1::big_ordered_map;
        use 0x5::order_book_types;
        pragma opaque = true, aborts_if_is_partial = false;
        let cse_ = OrderData {
            order_id: order_id,
            order_book_type: order_book_type,
            size: size
        };
        ensures [inferred]!result_of<is_taker_order>(old(self), price, is_bid) && is_bid ==>
            self
                == update_field(
                    old(self),
                    buys,
                    big_ordered_map::spec_set<PriceDescTime, OrderData>(
                        old(self).buys,
                        PriceDescTime {
                            price: price,
                            tie_breaker: order_book_types::into_decreasing_idx_type(
                                unique_priority_idx
                            )
                        },
                        cse_
                    )
                );
        ensures [inferred]!result_of<is_taker_order>(old(self), price, is_bid)
            && !is_bid ==>
            self
                == update_field(
                    old(self),
                    sells,
                    big_ordered_map::spec_set<PriceAscTime, OrderData>(
                        old(self).sells,
                        PriceAscTime { price: price, tie_breaker: unique_priority_idx },
                        cse_
                    )
                );
        ensures [inferred] result_of<is_taker_order>(old(self), price, is_bid) ==>
            self == old(self);
        aborts_if [inferred] result_of<is_taker_order>(self, price, is_bid);
        aborts_if [inferred]!result_of<is_taker_order>(self, price, is_bid)
            && (
                is_bid
                    && big_ordered_map::spec_aborts_add<PriceDescTime, OrderData>(
                        self.buys,
                        PriceDescTime {
                            price: price,
                            tie_breaker: order_book_types::into_decreasing_idx_type(
                                unique_priority_idx
                            )
                        },
                        cse_
                    )
            );
        aborts_if [inferred]!result_of<is_taker_order>(self, price, is_bid)
            && (
                is_bid
                    && aborts_of<order_book_types::into_decreasing_idx_type>(
                        unique_priority_idx
                    )
            );
        aborts_if [inferred]!result_of<is_taker_order>(self, price, is_bid)
            && (
                !is_bid
                    && big_ordered_map::spec_aborts_add<PriceAscTime, OrderData>(
                        self.sells,
                        PriceAscTime { price: price, tie_breaker: unique_priority_idx },
                        cse_
                    )
            );
    }
}
