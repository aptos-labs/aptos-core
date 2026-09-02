spec aptos_trading::order_match_types {

    spec destroy_active_matched_order(
        self: 0x5::order_match_types::ActiveMatchedOrder
    ): (
        0x5::order_book_types::OrderId,
        u64,
        u64,
        0x5::order_book_types::OrderType
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order_id;
        ensures [inferred] result_2 == self.matched_size;
        ensures [inferred] result_3 == self.remaining_size;
        ensures [inferred] result_4 == self.order_book_type;
        aborts_if [inferred] false;
    }

    spec destroy_order_match<M: copy + drop + store>(
        self: 0x5::order_match_types::OrderMatch<M>
    ): (0x5::order_match_types::OrderMatchDetails<M>, u64) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order;
        ensures [inferred] result_2 == self.matched_size;
        aborts_if [inferred] false;
    }

    spec destroy_single_order_match_details<M: copy + drop + store>(
        self: 0x5::order_match_types::OrderMatchDetails<M>
    ): (
        0x5::order_book_types::OrderId,
        address,
        0x1::option::Option<0x1::string::String>,
        0x5::order_book_types::IncreasingIdx,
        u64,
        u64,
        u64,
        bool,
        0x5::order_book_types::TimeInForce,
        u64,
        M
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order_id;
        ensures [inferred] result_2 == self.account;
        ensures [inferred] result_3 == self.client_order_id;
        ensures [inferred] result_4 == self.unique_priority_idx;
        ensures [inferred] result_5 == self.price;
        ensures [inferred] result_6 == self.orig_size;
        ensures [inferred] result_7 == self.remaining_size;
        ensures [inferred] result_8 == self.is_bid;
        ensures [inferred] result_9 == self.time_in_force;
        ensures [inferred] result_10 == self.creation_time_micros;
        ensures [inferred] result_11 == self.metadata;
        aborts_if [inferred] self is BulkOrder;
    }

    spec get_account_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): address {
        pragma opaque = true;
        ensures [inferred] result == self.account;
        aborts_if [inferred] false;
    }

    spec get_book_type_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): 0x5::order_book_types::OrderType {
        use 0x5::order_book_types;
        pragma opaque = true;
        ensures [inferred](self is SingleOrder) ==>
            result == order_book_types::single_order_type();
        ensures [inferred](self is BulkOrder) ==>
            result == order_book_types::bulk_order_type();
        aborts_if [inferred] false;
    }

    spec get_client_order_id_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): 0x1::option::Option<0x1::string::String> {
        use 0x1::option;
        use 0x1::string;
        pragma opaque = true;
        ensures [inferred](self is SingleOrder) ==>
            result == self.client_order_id;
        ensures [inferred](self is BulkOrder) ==>
            result == option::none<string::String>();
        aborts_if [inferred] false;
    }

    spec get_creation_time_micros_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==>
            result == self.creation_time_micros;
        ensures [inferred](self is SingleOrder) ==>
            result == self.creation_time_micros;
        aborts_if [inferred] false;
    }

    spec get_matched_size<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatch<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.matched_size;
        aborts_if [inferred] false;
    }

    spec get_metadata_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): M {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==> result == self.metadata;
        ensures [inferred](self is SingleOrder) ==>
            result == self.metadata;
        aborts_if [inferred] false;
    }

    spec get_order_id_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): 0x5::order_book_types::OrderId {
        pragma opaque = true;
        ensures [inferred] result == self.order_id;
        aborts_if [inferred] false;
    }

    spec get_orig_size_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.orig_size;
        aborts_if [inferred] self is BulkOrder;
    }

    spec get_price_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==> result == self.price;
        ensures [inferred](self is SingleOrder) ==> result == self.price;
        aborts_if [inferred] false;
    }

    spec get_remaining_size_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==>
            result == self.remaining_size;
        ensures [inferred](self is SingleOrder) ==>
            result == self.remaining_size;
        aborts_if [inferred] false;
    }

    spec get_sequence_number_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.sequence_number;
        aborts_if [inferred] self is SingleOrder;
    }

    spec get_time_in_force_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): 0x5::order_book_types::TimeInForce {
        use 0x5::order_book_types;
        pragma opaque = true;
        ensures [inferred](self is SingleOrder) ==>
            result == self.time_in_force;
        ensures [inferred](self is BulkOrder) ==>
            result == order_book_types::good_till_cancelled();
        aborts_if [inferred] false;
    }

    spec get_unique_priority_idx_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): 0x5::order_book_types::IncreasingIdx {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==>
            result == self.unique_priority_idx;
        ensures [inferred](self is SingleOrder) ==>
            result == self.unique_priority_idx;
        aborts_if [inferred] false;
    }

    spec is_active_matched_book_type_single_order(
        self: &0x5::order_match_types::ActiveMatchedOrder
    ): bool {
        use 0x5::order_book_types;
        pragma opaque = true;
        ensures [inferred] result
            == order_book_types::is_single_order_type(self.order_book_type);
        aborts_if [inferred] false;
    }

    spec is_bid_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder) ==> result == self.is_bid;
        ensures [inferred](self is SingleOrder) ==> result == self.is_bid;
        aborts_if [inferred] false;
    }

    spec is_bulk_order_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is BulkOrder);
        aborts_if [inferred] false;
    }

    spec is_single_order_from_match_details<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (self is SingleOrder);
        aborts_if [inferred] false;
    }

    spec new_active_matched_order(
        order_id: 0x5::order_book_types::OrderId,
        matched_size: u64,
        remaining_size: u64,
        order_book_type: 0x5::order_book_types::OrderType
    ): 0x5::order_match_types::ActiveMatchedOrder {
        pragma opaque = true;
        ensures [inferred] result
            == ActiveMatchedOrder::V1 {
                order_id: order_id,
                matched_size: matched_size,
                remaining_size: remaining_size,
                order_book_type: order_book_type
            };
        aborts_if [inferred] false;
    }

    spec new_bulk_order_match_details<M: copy + drop + store>(
        order_id: 0x5::order_book_types::OrderId,
        account: address,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        price: u64,
        remaining_size: u64,
        is_bid: bool,
        sequence_number: u64,
        creation_time_micros: u64,
        metadata: M
    ): 0x5::order_match_types::OrderMatchDetails<M> {
        pragma opaque = true;
        ensures [inferred] result
            == OrderMatchDetails::BulkOrder<M> {
                order_id: order_id,
                account: account,
                unique_priority_idx: unique_priority_idx,
                price: price,
                remaining_size: remaining_size,
                is_bid: is_bid,
                sequence_number: sequence_number,
                creation_time_micros: creation_time_micros,
                metadata: metadata
            };
        aborts_if [inferred] false;
    }

    spec new_order_match<M: copy + drop + store>(
        order: 0x5::order_match_types::OrderMatchDetails<M>, matched_size: u64
    ): 0x5::order_match_types::OrderMatch<M> {
        pragma opaque = true;
        ensures [inferred] result
            == OrderMatch::V1<M> { order: order, matched_size: matched_size };
        aborts_if [inferred] false;
    }

    spec new_order_match_details_with_modified_size<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>, remaining_size: u64
    ): 0x5::order_match_types::OrderMatchDetails<M> {
        ensures result == update_field(self, remaining_size, remaining_size);
        aborts_if false;
    }

    spec new_single_order_match_details<M: copy + drop + store>(
        order_id: 0x5::order_book_types::OrderId,
        account: address,
        client_order_id: 0x1::option::Option<0x1::string::String>,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: 0x5::order_book_types::TimeInForce,
        creation_time_micros: u64,
        metadata: M
    ): 0x5::order_match_types::OrderMatchDetails<M> {
        pragma opaque = true;
        ensures [inferred] result
            == OrderMatchDetails::SingleOrder<M> {
                order_id: order_id,
                account: account,
                client_order_id: client_order_id,
                unique_priority_idx: unique_priority_idx,
                price: price,
                orig_size: orig_size,
                remaining_size: remaining_size,
                is_bid: is_bid,
                time_in_force: time_in_force,
                creation_time_micros: creation_time_micros,
                metadata: metadata
            };
        aborts_if [inferred] false;
    }

    spec validate_bulk_order_reinsertion_request<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>,
        other: &0x5::order_match_types::OrderMatchDetails<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred](self is BulkOrder)
            && ((other is BulkOrder)
                && (self.order_id == other.order_id
                    && self.account != other.account)) ==> result == false;
        ensures [inferred](self is BulkOrder)
            && (
                (other is BulkOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && self.unique_priority_idx
                                        != other.unique_priority_idx
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is BulkOrder)
            && (
                (other is BulkOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && self.price != other.price
                                    )
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is BulkOrder)
            && (
                (other is BulkOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && (
                                                self.price == other.price
                                                    && self.is_bid == other.is_bid
                                            )
                                    )
                            )
                    )
            ) ==>
            result == (self.sequence_number == other.sequence_number);
        ensures [inferred](self is BulkOrder)
            && (
                (other is BulkOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && (
                                                self.price == other.price
                                                    && self.is_bid != other.is_bid
                                            )
                                    )
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is BulkOrder)
            && ((other is BulkOrder)
                && self.order_id != other.order_id) ==> result == false;
        aborts_if [inferred] self is SingleOrder;
        aborts_if [inferred](self is BulkOrder) && (other is SingleOrder);
    }

    spec validate_single_order_reinsertion_request<M: copy + drop + store>(
        self: &0x5::order_match_types::OrderMatchDetails<M>,
        other: &0x5::order_match_types::OrderMatchDetails<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred](self is SingleOrder)
            && ((other is SingleOrder)
                && (self.order_id == other.order_id
                    && self.account != other.account)) ==> result == false;
        ensures [inferred](self is SingleOrder)
            && (
                (other is SingleOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && self.unique_priority_idx
                                        != other.unique_priority_idx
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is SingleOrder)
            && (
                (other is SingleOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && (
                                                self.price == other.price
                                                    && self.orig_size
                                                        != other.orig_size
                                            )
                                    )
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is SingleOrder)
            && (
                (other is SingleOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && (
                                                self.price == other.price
                                                    && self.orig_size
                                                        == other.orig_size
                                            )
                                    )
                            )
                    )
            ) ==>
            result == (self.is_bid == other.is_bid);
        ensures [inferred](self is SingleOrder)
            && (
                (other is SingleOrder)
                    && (
                        self.order_id == other.order_id
                            && (
                                self.account == other.account
                                    && (
                                        self.unique_priority_idx
                                            == other.unique_priority_idx
                                            && self.price != other.price
                                    )
                            )
                    )
            ) ==> result == false;
        ensures [inferred](self is SingleOrder)
            && ((other is SingleOrder)
                && self.order_id != other.order_id) ==> result == false;
        aborts_if [inferred] self is BulkOrder;
        aborts_if [inferred](self is SingleOrder) && (other is BulkOrder);
    }
}
