/// Order book type definitions
module aptos_experimental::order_book_types {
    use std::option;
    use std::option::Option;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};

    friend aptos_experimental::price_time_index;
    friend aptos_experimental::single_order_book;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::market;
    friend aptos_experimental::order_book;
    friend aptos_experimental::single_order_types;
    friend aptos_experimental::bulk_order_book;
    friend aptos_experimental::bulk_order_book_types;
    #[test_only]
    friend aptos_experimental::bulk_order_book_tests;
    #[test_only]
    friend aptos_experimental::order_book_client_order_id;

    const U128_MAX: u128 = 0xffffffffffffffffffffffffffffffff;

    const BIG_MAP_INNER_DEGREE: u16 = 64;
    const BIG_MAP_LEAF_DEGREE: u16 = 32;

    // to replace types:
    struct OrderIdType has store, copy, drop {
        order_id: u128
    }

    struct AccountClientOrderId has store, copy, drop {
        account: address,
        client_order_id: u64
    }

    // Internal type representing order in which trades are placed.
    struct UniqueIdxType has store, copy, drop {
        idx: u128
    }

    enum OrderBookType has store, drop, copy {
        SingleOrderBook,
        BulkOrderBook
    }

    public fun single_order_book_type(): OrderBookType {
        OrderBookType::SingleOrderBook
    }

    public fun bulk_order_book_type(): OrderBookType {
        OrderBookType::BulkOrderBook
    }

    public(friend) fun new_default_big_ordered_map<K: store, V: store>(): BigOrderedMap<K, V> {
        big_ordered_map::new_with_config(
            BIG_MAP_INNER_DEGREE,
            BIG_MAP_LEAF_DEGREE,
            true
        )
    }

    public fun new_order_id_type(order_id: u128): OrderIdType {
        OrderIdType { order_id }
    }

    public fun new_account_client_order_id(
        account: address, client_order_id: u64
    ): AccountClientOrderId {
        AccountClientOrderId { account, client_order_id }
    }

    public(friend) fun new_unique_idx_type(idx: u128): UniqueIdxType {
        UniqueIdxType { idx }
    }

    public(friend) fun descending_idx(self: &UniqueIdxType): UniqueIdxType {
        UniqueIdxType { idx: U128_MAX - self.idx }
    }

    public fun get_order_id_value(self: &OrderIdType): u128 {
        self.order_id
    }

    const EINVALID_TIME_IN_FORCE: u64 = 5;

    /// Order time in force
    enum TimeInForce has drop, copy, store {
        /// Good till cancelled order type
        GTC,
        /// Post Only order type - ensures that the order is not a taker order
        POST_ONLY,
        /// Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
        /// order as possible as taker order and cancel the rest.
        IOC
    }

    public fun time_in_force_from_index(index: u8): TimeInForce {
        if (index == 0) {
            TimeInForce::GTC
        } else if (index == 1) {
            TimeInForce::POST_ONLY
        } else if (index == 2) {
            TimeInForce::IOC
        } else {
            abort EINVALID_TIME_IN_FORCE
        }
    }

    #[test_only]
    public fun time_in_force_to_index(self: &TimeInForce): u8 {
        match (self) {
            GTC => 0,
            POST_ONLY => 1,
            IOC => 2,
        }
    }

    public fun good_till_cancelled(): TimeInForce {
        TimeInForce::GTC
    }

    public fun post_only(): TimeInForce {
        TimeInForce::POST_ONLY
    }

    public fun immediate_or_cancel(): TimeInForce {
        TimeInForce::IOC
    }

    enum TriggerCondition has store, drop, copy {
        PriceMoveAbove(u64),
        PriceMoveBelow(u64),
        TimeBased(u64)
    }

    public fun new_time_based_trigger_condition(time: u64): TriggerCondition {
        TriggerCondition::TimeBased(time)
    }

    public fun price_move_up_condition(price: u64): TriggerCondition {
        TriggerCondition::PriceMoveAbove(price)
    }

    public fun price_move_down_condition(price: u64): TriggerCondition {
        TriggerCondition::PriceMoveBelow(price)
    }

    // Returns the price move down index and price move up index for a particular trigger condition
    public fun index(self: &TriggerCondition):
        (option::Option<u64>, option::Option<u64>, option::Option<u64>) {
        match(self) {
            TriggerCondition::PriceMoveAbove(price) => {
                (option::none(), option::some(*price), option::none())
            }
            TriggerCondition::PriceMoveBelow(price) => {
                (option::some(*price), option::none(), option::none())
            }
            TriggerCondition::TimeBased(time) => {
                (option::none(), option::none(), option::some(*time))
            }
        }
    }

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
        V1 {
            order_id: OrderIdType,
            account: address,
            client_order_id: Option<u64>, // for client to track orders
            unique_priority_idx: UniqueIdxType,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            time_in_force: TimeInForce,
            metadata: Option<M>,
            order_book_type: OrderBookType
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

    public(friend) fun destroy_order_match<M: store + copy + drop>(
        self: OrderMatch<M>,
    ): (OrderMatchDetails<M>, u64) {
        let OrderMatch::V1 { order, matched_size } = self;
        (order, matched_size)
    }

    public(friend) fun destroy_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>,
    ): (OrderIdType, address, Option<u64>, UniqueIdxType, u64, u64, u64, bool, TimeInForce, Option<M>, OrderBookType) {
        let OrderMatchDetails::V1 {
            order_id,
            account,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            metadata,
            order_book_type
        } = self;
        (order_id, account, client_order_id, unique_priority_idx, price, orig_size, remaining_size, is_bid, time_in_force,  metadata, order_book_type)
    }

    public fun get_matched_size<M: store + copy + drop>(
        self: &OrderMatch<M>,
    ): u64 {
        self.matched_size
    }

    /// Validates that a reinsertion request is valid for the given original order.
    ///


    public(friend) fun get_account_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): address {
        self.account
    }

    public(friend) fun get_order_id_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): OrderIdType {
        self.order_id
    }

    public(friend) fun get_unique_priority_idx_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): UniqueIdxType {
        self.unique_priority_idx
    }

    public(friend) fun get_price_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        self.price
    }

    public(friend) fun get_orig_size_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        self.orig_size
    }

    public(friend) fun get_remaining_size_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        self.remaining_size
    }

    public(friend) fun get_time_in_force_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): TimeInForce {
        self.time_in_force
    }

    public(friend) fun get_metadata_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): Option<M> {
        self.metadata
    }

    public(friend) fun get_client_order_id_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): Option<u64> {
        self.client_order_id
    }

    public(friend) fun is_bid_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): bool {
        self.is_bid
    }

    public(friend) fun get_book_type_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): OrderBookType {
        self.order_book_type
    }


    public(friend) fun new_order_match_details<M: store + copy + drop>(
        order_id: OrderIdType,
        account: address,
        client_order_id: Option<u64>,
        unique_priority_idx: UniqueIdxType,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        metadata: Option<M>,
        order_book_type: OrderBookType
    ): OrderMatchDetails<M> {
        OrderMatchDetails::V1 {
            order_id,
            account,
            client_order_id,
            unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            metadata,
            order_book_type
        }
    }

    public fun new_order_match_details_with_modified_size<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
        remaining_size: u64
    ): OrderMatchDetails<M> {
        OrderMatchDetails::V1 {
            order_id: self.order_id,
            account: self.account,
            client_order_id: self.client_order_id,
            unique_priority_idx: self.unique_priority_idx,
            price: self.price,
            orig_size: self.orig_size,
            remaining_size,
            is_bid: self.is_bid,
            time_in_force: self.time_in_force,
            metadata: self.metadata,
            order_book_type: self.order_book_type
        }
    }

    public(friend) fun new_order_match<M: store + copy + drop>(
        order: OrderMatchDetails<M>,
        matched_size: u64
    ): OrderMatch<M> {
        OrderMatch::V1 {
            order,
            matched_size
        }
    }

    public(friend) fun validate_reinsertion_request<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
        other: &OrderMatchDetails<M>,
    ): bool {
        self.order_id == other.order_id &&
        self.account == other.account &&
        self.unique_priority_idx == other.unique_priority_idx &&
        self.price == other.price &&
        self.orig_size == other.orig_size &&
        self.is_bid == other.is_bid
    }

    struct ActiveMatchedOrder has copy, drop {
        order_id: OrderIdType,
        matched_size: u64,
        /// Remaining size of the maker order
        remaining_size: u64,
        order_book_type: OrderBookType,
    }

    public(friend) fun new_active_matched_order(
        order_id: OrderIdType, matched_size: u64, remaining_size: u64, order_book_type: OrderBookType
    ): ActiveMatchedOrder {
        ActiveMatchedOrder { order_id, matched_size, remaining_size, order_book_type }
    }

    public(friend) fun destroy_active_matched_order(
        self: ActiveMatchedOrder
    ): (OrderIdType, u64, u64, OrderBookType) {
        (self.order_id, self.matched_size, self.remaining_size, self.order_book_type)
    }

    public(friend) fun get_active_matched_size(self: &ActiveMatchedOrder): u64 {
        self.matched_size
    }

    public(friend) fun get_active_matched_book_type(
        self: &ActiveMatchedOrder
    ): OrderBookType {
        self.order_book_type
    }

    public fun destroy_active_match_order(self: ActiveMatchedOrder): (OrderIdType, u64, u64) {
        (self.order_id, self.matched_size, self.remaining_size)
    }
}
