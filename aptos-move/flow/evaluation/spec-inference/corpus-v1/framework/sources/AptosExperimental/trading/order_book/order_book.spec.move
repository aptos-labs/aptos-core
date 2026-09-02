spec aptos_experimental::order_book {

    spec cancel_bulk_order<M: copy + drop + store>(
        self: &mut 0x7::order_book::OrderBook<M>, order_creator: address
    ): 0x5::bulk_order_types::BulkOrder<M> {
        use 0x7::bulk_order_book;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] result
            == result_of<bulk_order_book::cancel_bulk_order<M>> (
                old(self).bulk_order_book, old(self).price_time_idx, order_creator
            );
        ensures [inferred] self
            == update_field(
                update_field(old(self), bulk_order_book, self.bulk_order_book),
                price_time_idx,
                self.price_time_idx
            );
        ensures [inferred] ensures_of<bulk_order_book::cancel_bulk_order<M>> (
            update_field(
                update_field(
                    old(self),
                    bulk_order_book,
                    update_field(
                        update_field(
                            old(self),
                            bulk_order_book,
                            update_field(
                                update_field(
                                    old(self), bulk_order_book, self.bulk_order_book
                                ),
                                price_time_idx,
                                self.price_time_idx
                            ).bulk_order_book
                        ),
                        price_time_idx,
                        self.price_time_idx
                    ).bulk_order_book
                ),
                price_time_idx,
                self.price_time_idx
            ).bulk_order_book,
            update_field(
                update_field(
                    old(self),
                    bulk_order_book,
                    update_field(
                        update_field(
                            old(self),
                            bulk_order_book,
                            update_field(
                                update_field(
                                    old(self), bulk_order_book, self.bulk_order_book
                                ),
                                price_time_idx,
                                self.price_time_idx
                            ).bulk_order_book
                        ),
                        price_time_idx,
                        self.price_time_idx
                    ).bulk_order_book
                ),
                price_time_idx,
                self.price_time_idx
            ).price_time_idx,
            order_creator,
            result,
            update_field(
                update_field(
                    old(self),
                    bulk_order_book,
                    update_field(
                        update_field(
                            old(self),
                            bulk_order_book,
                            update_field(
                                update_field(
                                    old(self), bulk_order_book, self.bulk_order_book
                                ),
                                price_time_idx,
                                self.price_time_idx
                            ).bulk_order_book
                        ),
                        price_time_idx,
                        self.price_time_idx
                    ).bulk_order_book
                ),
                price_time_idx,
                self.price_time_idx
            ).bulk_order_book,
            self.price_time_idx
        );
        aborts_if aborts_of<bulk_order_book::cancel_bulk_order<M>> (
            self.bulk_order_book, self.price_time_idx, order_creator
        );
    }

    spec get_bulk_order_remaining_size<M: copy + drop + store>(
        self: &0x7::order_book::OrderBook<M>, order_creator: address, is_bid: bool
    ): u64 {
        use 0x7::bulk_order_book;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] result
            == result_of<bulk_order_book::get_remaining_size<M>> (
                self.bulk_order_book, order_creator, is_bid
            );
        aborts_if aborts_of<bulk_order_book::get_remaining_size<M>> (
            self.bulk_order_book, order_creator, is_bid
        );
    }

    spec place_bulk_order<M: copy + drop + store>(
        self: &mut 0x7::order_book::OrderBook<M>,
        order_req: 0x5::bulk_order_types::BulkOrderRequest<M>
    ): 0x5::bulk_order_types::BulkOrderPlaceResponse<M> {
        use 0x7::bulk_order_book;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures ensures_of<bulk_order_book::place_bulk_order<M>> (
            old(self).bulk_order_book,
            old(self).price_time_idx,
            order_req,
            result,
            self.bulk_order_book,
            self.price_time_idx
        );
        ensures self
            == update_field(
                update_field(old(self), bulk_order_book, self.bulk_order_book),
                price_time_idx,
                self.price_time_idx
            );
        aborts_if aborts_of<bulk_order_book::place_bulk_order<M>> (
            self.bulk_order_book, self.price_time_idx, order_req
        );
    }
}
