module aptos_trading::order_match_types {
    friend aptos_trading::single_order_types;

    use aptos_trading::order_book_types::{
        OrderId,
        OrderType,
        IncreasingIdx,
        TimeInForce,
        good_till_cancelled,
        single_order_type,
        bulk_order_type,
        is_single_order_type
    };
    use std::option::{Option, Self};
    use std::string::String;

    const E_REINSERT_ORDER_MISMATCH: u64 = 8;

    /// Represents the details of a matched order.
    ///
    /// Contains information about an order that was matched, including its
    /// identifier, account, priority index, price, sizes, and side.
    ///
    /// # Fields:
    /// - `order_id`: Unique identifier for the order
    /// - `account`: Account that placed the order
    /// - `unique_priority_idx`: Priority index for time-based ordering
    /// - `price`: Price at which the order was matched
    /// - `orig_size`: Original size of the order
    /// - `remaining_size`: Remaining size after the match
    /// - `is_bid`: True if this was a bid order, false if ask order
    enum OrderMatchDetails<M: store + copy + drop> has copy, drop {
        SingleOrder {
            order_id: OrderId,
            account: address,
            client_order_id: Option<String>, // for client to track orders
            unique_priority_idx: IncreasingIdx,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            time_in_force: TimeInForce,
            creation_time_micros: u64,
            metadata: M
        },
        BulkOrder {
            order_id: OrderId,
            account: address,
            unique_priority_idx: IncreasingIdx,
            price: u64,
            remaining_size: u64,
            is_bid: bool,
            sequence_number: u64,
            creation_time_micros: u64,
            metadata: M
        }
    }

    /// Represents a single match between a taker order and a maker order.
    ///
    /// Contains the matched order details and the size that was matched in this
    /// particular match operation.
    ///
    /// # Fields:
    /// - `order`: The matched order result
    /// - `matched_size`: The size that was matched in this operation
    enum OrderMatch<M: store + copy + drop> has drop, copy {
        V1 {
            order: OrderMatchDetails<M>,
            matched_size: u64
        }
    }

    enum ActiveMatchedOrder has copy, drop {
        V1 {
            order_id: OrderId,
            matched_size: u64,
            /// Remaining size of the maker order
            remaining_size: u64,
            order_book_type: OrderType
        }
    }

    public fun new_single_order_match_details<M: store + copy + drop>(
        order_id: OrderId,
        account: address,
        client_order_id: Option<String>,
        unique_priority_idx: IncreasingIdx,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        creation_time_micros: u64,
        metadata: M
    ): OrderMatchDetails<M> {
        OrderMatchDetails::SingleOrder {
            order_id,
            account,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            creation_time_micros,
            metadata
        }
    }

    public fun new_bulk_order_match_details<M: store + copy + drop>(
        order_id: OrderId,
        account: address,
        unique_priority_idx: IncreasingIdx,
        price: u64,
        remaining_size: u64,
        is_bid: bool,
        sequence_number: u64,
        creation_time_micros: u64,
        metadata: M
    ): OrderMatchDetails<M> {
        OrderMatchDetails::BulkOrder {
            order_id,
            account,
            unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            sequence_number,
            creation_time_micros,
            metadata
        }
    }

    public fun new_order_match<M: store + copy + drop>(
        order: OrderMatchDetails<M>, matched_size: u64
    ): OrderMatch<M> {
        OrderMatch::V1 { order, matched_size }
    }

    public fun new_order_match_details_with_modified_size<M: store + copy + drop>(
        self: &OrderMatchDetails<M>, remaining_size: u64
    ): OrderMatchDetails<M> {
        let res = *self;
        res.remaining_size = remaining_size;
        res
    }

    public fun new_active_matched_order(
        order_id: OrderId,
        matched_size: u64,
        remaining_size: u64,
        order_book_type: OrderType
    ): ActiveMatchedOrder {
        ActiveMatchedOrder::V1 { order_id, matched_size, remaining_size, order_book_type }
    }

    public fun get_matched_size<M: store + copy + drop>(self: &OrderMatch<M>): u64 {
        self.matched_size
    }

    /// Validates that a reinsertion request is valid for the given original order.
    ///
    public fun get_account_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): address {
        self.account
    }

    public fun get_order_id_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): OrderId {
        self.order_id
    }

    public fun get_unique_priority_idx_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): IncreasingIdx {
        self.unique_priority_idx
    }

    public fun get_price_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): u64 {
        self.price
    }

    public fun get_orig_size_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): u64 {
        self.orig_size
    }

    public fun get_remaining_size_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): u64 {
        self.remaining_size
    }

    public fun get_time_in_force_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): TimeInForce {
        if (self is OrderMatchDetails::SingleOrder) {
            self.time_in_force
        } else {
            good_till_cancelled()
        }
    }

    public fun get_metadata_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): M {
        self.metadata
    }

    public fun get_client_order_id_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): Option<String> {
        if (self is OrderMatchDetails::SingleOrder) {
            self.client_order_id
        } else {
            option::none()
        }
    }

    public fun is_bid_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): bool {
        self.is_bid
    }

    public fun get_book_type_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): OrderType {
        if (self is OrderMatchDetails::SingleOrder) {
            single_order_type()
        } else {
            bulk_order_type()
        }
    }

    public fun is_bulk_order_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): bool {
        self is OrderMatchDetails::BulkOrder
    }

    public fun is_single_order_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): bool {
        self is OrderMatchDetails::SingleOrder
    }

    /// This should only be called on bulk orders, aborts if called for non-bulk order.
    public fun get_sequence_number_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): u64 {
        self.sequence_number
    }

    public fun get_creation_time_micros_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>
    ): u64 {
        self.creation_time_micros
    }

    public fun destroy_order_match<M: store + copy + drop>(
        self: OrderMatch<M>
    ): (OrderMatchDetails<M>, u64) {
        let OrderMatch::V1 { order, matched_size } = self;
        (order, matched_size)
    }

    public fun destroy_single_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>
    ): (
        OrderId,
        address,
        Option<String>,
        IncreasingIdx,
        u64,
        u64,
        u64,
        bool,
        TimeInForce,
        u64,
        M
    ) {
        let OrderMatchDetails::SingleOrder {
            order_id,
            account,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            creation_time_micros,
            metadata
        } = self;
        (
            order_id,
            account,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            creation_time_micros,
            metadata
        )
    }

    #[test_only]
    public fun destroy_bulk_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>
    ): (OrderId, address, IncreasingIdx, u64, u64, bool, u64, u64, M) {
        let OrderMatchDetails::BulkOrder {
            order_id,
            account,
            unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            sequence_number,
            creation_time_micros,
            metadata
        } = self;
        (
            order_id,
            account,
            unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            sequence_number,
            creation_time_micros,
            metadata
        )
    }

    public fun validate_single_order_reinsertion_request<M: store + copy + drop>(
        self: &OrderMatchDetails<M>, other: &OrderMatchDetails<M>
    ): bool {
        assert!(self is OrderMatchDetails::SingleOrder, E_REINSERT_ORDER_MISMATCH);
        assert!(other is OrderMatchDetails::SingleOrder, E_REINSERT_ORDER_MISMATCH);

        self.order_id == other.order_id
            && self.account == other.account
            && self.unique_priority_idx == other.unique_priority_idx
            && self.price == other.price
            && self.orig_size == other.orig_size
            && self.is_bid == other.is_bid
    }

    public fun validate_bulk_order_reinsertion_request<M: store + copy + drop>(
        self: &OrderMatchDetails<M>, other: &OrderMatchDetails<M>
    ): bool {
        assert!(self is OrderMatchDetails::BulkOrder, E_REINSERT_ORDER_MISMATCH);
        assert!(other is OrderMatchDetails::BulkOrder, E_REINSERT_ORDER_MISMATCH);

        self.order_id == other.order_id
            && self.account == other.account
            && self.unique_priority_idx == other.unique_priority_idx
            && self.price == other.price
            && self.is_bid == other.is_bid
            && self.sequence_number == other.sequence_number
    }

    public fun destroy_active_matched_order(self: ActiveMatchedOrder)
        : (OrderId, u64, u64, OrderType) {
        let ActiveMatchedOrder::V1 {
            order_id,
            matched_size,
            remaining_size,
            order_book_type
        } = self;
        (order_id, matched_size, remaining_size, order_book_type)
    }

    #[test_only]
    public fun get_active_matched_size(self: &ActiveMatchedOrder): u64 {
        self.matched_size
    }

    public fun is_active_matched_book_type_single_order(
        self: &ActiveMatchedOrder
    ): bool {
        is_single_order_type(&self.order_book_type)
    }
}
