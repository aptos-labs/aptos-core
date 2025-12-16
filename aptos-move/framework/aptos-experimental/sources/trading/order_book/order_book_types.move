/// Order book type definitions
module aptos_experimental::order_book_types {
    friend aptos_experimental::order_book;
    friend aptos_experimental::single_order_book;
    friend aptos_experimental::bulk_order_book;
    friend aptos_experimental::price_time_index;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::order_placement;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::market_types;
    friend aptos_experimental::market_bulk_order;
    friend aptos_experimental::single_order_types;
    friend aptos_experimental::bulk_order_book_types;
    #[test_only] friend aptos_experimental::bulk_order_book_tests;
    #[test_only] friend aptos_experimental::order_book_client_order_id;
    use std::option;
    use std::option::Option;
    use std::string::String;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::transaction_context;

    const U128_MAX: u128 = 0xffffffffffffffffffffffffffffffff;

    const BIG_MAP_INNER_DEGREE: u16 = 64;
    const BIG_MAP_LEAF_DEGREE: u16 = 32;

    const SINGLE_ORDER_TYPE: u16 = 0;
    const BULK_ORDER_TYPE: u16 = 1;

    // to replace types:
    struct OrderIdType has store, copy, drop {
        order_id: u128
    }

    struct AccountClientOrderId has store, copy, drop {
        account: address,
        client_order_id: String
    }

    // Internal type representing order in which trades are placed.
    struct UniqueIdxType has store, copy, drop {
        idx: u128
    }

    struct OrderType has store, drop, copy {
        // Represented as in integer to keep constant size enumeration, suitable to use efficiently in
        // data structures such as big ordered map, etc.
        type: u16
    }

    public fun single_order_type(): OrderType {
        OrderType { type: SINGLE_ORDER_TYPE }
    }

    public fun bulk_order_type(): OrderType {
        OrderType { type: BULK_ORDER_TYPE }
    }

    public fun is_bulk_order_type(order_type: &OrderType): bool {
        order_type.type == BULK_ORDER_TYPE
    }

    public fun is_single_order_type(order_type: &OrderType): bool {
        order_type.type == SINGLE_ORDER_TYPE
    }

    public(friend) fun new_default_big_ordered_map<K: store, V: store>(): BigOrderedMap<K, V> {
        big_ordered_map::new_with_config(
            BIG_MAP_INNER_DEGREE,
            BIG_MAP_LEAF_DEGREE,
            true
        )
    }

    public fun next_order_id(): OrderIdType {
        // reverse bits to make order ids random, so indices on top of them are shuffled.
        OrderIdType { order_id: reverse_bits(transaction_context::monotonically_increasing_counter()) }
    }

    public fun new_order_id_type(order_id: u128): OrderIdType {
        OrderIdType { order_id }
    }

    public fun new_account_client_order_id(
        account: address, client_order_id: String
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
    const E_REINSERT_ORDER_MISMATCH: u64 = 8;

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

    // The time should be seconds since unix epoch
    public fun new_time_based_trigger_condition(time_secs: u64): TriggerCondition {
        TriggerCondition::TimeBased(time_secs)
    }

    public fun price_move_up_condition(price: u64): TriggerCondition {
        TriggerCondition::PriceMoveAbove(price)
    }

    public fun price_move_down_condition(price: u64): TriggerCondition {
        TriggerCondition::PriceMoveBelow(price)
    }

    // Returns the price move down index and price move up index for a particular trigger condition
    public(friend) fun index(self: &TriggerCondition):
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
        SingleOrder {
            order_id: OrderIdType,
            account: address,
            client_order_id: Option<String>, // for client to track orders
            unique_priority_idx: UniqueIdxType,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            time_in_force: TimeInForce,
            creation_time_micros: u64,
            metadata: M,
        },
        BulkOrder {
            order_id: OrderIdType,
            account: address,
            unique_priority_idx: UniqueIdxType,
            price: u64,
            remaining_size: u64,
            is_bid: bool,
            sequence_number: u64,
            creation_time_micros: u64,
            metadata: M,
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

    public(friend) fun destroy_single_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>,
    ): (OrderIdType, address, Option<String>, UniqueIdxType, u64, u64, u64, bool, TimeInForce, u64, M) {
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
            metadata,
        } = self;
        (order_id, account, client_order_id, unique_priority_idx, price, orig_size, remaining_size, is_bid, time_in_force, creation_time_micros, metadata)
    }

    public(friend) fun destroy_bulk_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>,
    ): (OrderIdType, address, UniqueIdxType, u64, u64, bool, u64, u64, M) {
        let OrderMatchDetails::BulkOrder {
            order_id,
            account,
            unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            sequence_number,
            creation_time_micros,
            metadata,
        } = self;
        (order_id, account, unique_priority_idx, price, remaining_size, is_bid, sequence_number, creation_time_micros, metadata)
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
        if (self is OrderMatchDetails::SingleOrder) {
            self.time_in_force
        } else {
            good_till_cancelled()
        }
    }

    public(friend) fun get_metadata_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): M {
        self.metadata
    }

    public(friend) fun get_client_order_id_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): Option<String> {
        if (self is OrderMatchDetails::SingleOrder) {
            return self.client_order_id
        } else {
            return option::none()
        }
    }

    public(friend) fun is_bid_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): bool {
        self.is_bid
    }

    public(friend) fun get_book_type_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): OrderType {
        if (self is OrderMatchDetails::SingleOrder) {
            single_order_type()
        } else {
            bulk_order_type()
        }
    }

    public(friend) fun is_bulk_order_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): bool {
        self is OrderMatchDetails::BulkOrder
    }

    public(friend) fun is_single_order_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): bool {
        self is OrderMatchDetails::SingleOrder
    }


    /// This should only be called on bulk orders, aborts if called for non-bulk order.
    public(friend) fun get_sequence_number_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        self.sequence_number
    }

    public(friend) fun get_creation_time_micros_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        self.creation_time_micros
    }

    public(friend) fun new_single_order_match_details<M: store + copy + drop>(
        order_id: OrderIdType,
        account: address,
        client_order_id: Option<String>,
        unique_priority_idx: UniqueIdxType,
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
            metadata,
        }
    }

    public(friend) fun new_bulk_order_match_details<M: store + copy + drop>(
        order_id: OrderIdType,
        account: address,
        unique_priority_idx: UniqueIdxType,
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
            metadata,
        }
    }

    public(friend) fun new_order_match_details_with_modified_size<M: store + copy + drop>(
        self: OrderMatchDetails<M>,
        remaining_size: u64
    ): OrderMatchDetails<M> {
        self.remaining_size = remaining_size;
        self
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

    public(friend) fun validate_single_order_reinsertion_request<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
        other: &OrderMatchDetails<M>,
    ): bool {
        assert!(self is OrderMatchDetails::SingleOrder, E_REINSERT_ORDER_MISMATCH);
        assert!(other is OrderMatchDetails::SingleOrder, E_REINSERT_ORDER_MISMATCH);

        self.order_id == other.order_id &&
        self.account == other.account &&
        self.unique_priority_idx == other.unique_priority_idx &&
        self.price == other.price &&
        self.orig_size == other.orig_size &&
        self.is_bid == other.is_bid
    }

    public(friend) fun validate_bulk_order_reinsertion_request<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
        other: &OrderMatchDetails<M>,
    ): bool {
        assert!(self is OrderMatchDetails::BulkOrder, E_REINSERT_ORDER_MISMATCH);
        assert!(other is OrderMatchDetails::BulkOrder, E_REINSERT_ORDER_MISMATCH);

        self.order_id == other.order_id &&
        self.account == other.account &&
        self.unique_priority_idx == other.unique_priority_idx &&
        self.price == other.price &&
        self.is_bid == other.is_bid &&
        self.sequence_number == other.sequence_number
    }

    struct ActiveMatchedOrder has copy, drop {
        order_id: OrderIdType,
        matched_size: u64,
        /// Remaining size of the maker order
        remaining_size: u64,
        order_book_type: OrderType,
    }

    public(friend) fun new_active_matched_order(
        order_id: OrderIdType, matched_size: u64, remaining_size: u64, order_book_type: OrderType
    ): ActiveMatchedOrder {
        ActiveMatchedOrder { order_id, matched_size, remaining_size, order_book_type }
    }

    public(friend) fun destroy_active_matched_order(
        self: ActiveMatchedOrder
    ): (OrderIdType, u64, u64, OrderType) {
        let ActiveMatchedOrder { order_id, matched_size, remaining_size, order_book_type } = self;
        (order_id, matched_size, remaining_size, order_book_type)
    }

    public(friend) fun get_active_matched_size(self: &ActiveMatchedOrder): u64 {
        self.matched_size
    }

    public(friend) fun is_active_matched_book_type_single_order(
        self: &ActiveMatchedOrder
    ): bool {
        is_single_order_type(&self.order_book_type)
    }



    /// Reverse the bits in a u128 value using divide and conquer approach
    /// This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)
    fun reverse_bits(value: u128): u128 {
        let v = value;

        // Swap odd and even bits
        v = ((v & 0x55555555555555555555555555555555) << 1) | ((v >> 1) & 0x55555555555555555555555555555555);

        // Swap consecutive pairs
        v = ((v & 0x33333333333333333333333333333333) << 2) | ((v >> 2) & 0x33333333333333333333333333333333);

        // Swap nibbles
        v = ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) << 4) | ((v >> 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

        // Swap bytes
        v = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8) | ((v >> 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

        // Swap 2-byte chunks
        v = ((v & 0x0000ffff0000ffff0000ffff0000ffff) << 16) | ((v >> 16) & 0x0000ffff0000ffff0000ffff0000ffff);

        // Swap 4-byte chunks
        v = ((v & 0x00000000ffffffff00000000ffffffff) << 32) | ((v >> 32) & 0x00000000ffffffff00000000ffffffff);

        // Swap 8-byte chunks
        v = (v << 64) | (v >> 64);

        v
    }


    // ============================= Tests ====================================

    #[test]
    fun test_reverse_bits_order_id_type() {
        // Test basic bit reversal functionality
        let order_id_1 = 1;
        let order_id_2 = 2;
        let order_id_3 = 0x12345678;
        let order_id_4 = 0x87654321ABCDEF00;

        let reversed_1 = reverse_bits(order_id_1);
        let reversed_2 = reverse_bits(order_id_2);
        let reversed_3 = reverse_bits(order_id_3);
        let reversed_4 = reverse_bits(order_id_4);

        // Test that conversion back gives original value
        let recovered_1 = reverse_bits(reversed_1);
        let recovered_2 = reverse_bits(reversed_2);
        let recovered_3 = reverse_bits(reversed_3);
        let recovered_4 = reverse_bits(reversed_4);

        assert!(order_id_1 == recovered_1);
        assert!(order_id_2 == recovered_2);
        assert!(order_id_3 == recovered_3);
        assert!(order_id_4 == recovered_4);

        // Test that reversed values are different from originals (for non-palindromic bit patterns)
        // Now we can access the internal field since we're in the same module
        assert!(reversed_1 != order_id_1);
        assert!(reversed_2 != order_id_2);
        assert!(reversed_3 != order_id_3);
        assert!(reversed_4 != order_id_4);

        // Test specific bit reversal cases
        // 1 in binary: 0...0001, reversed should be 1000...0000 (high bit set)
        assert!(reversed_1 == (1u128 << 127));

        // 2 in binary: 0...0010, reversed should be 0100...0000
        assert!(reversed_2 == (1u128 << 126));

        // Test edge cases
        let order_id_zero = 0;
        let reversed_zero = reverse_bits(order_id_zero);
        let recovered_zero = reverse_bits(reversed_zero);
        assert!(order_id_zero == recovered_zero);
        assert!(reversed_zero == 0); // 0 reversed is still 0

        // Test maximum value
        let order_id_max = 0xffffffffffffffffffffffffffffffff;
        let reversed_max = reverse_bits(order_id_max);
        let recovered_max = reverse_bits(reversed_max);
        assert!(order_id_max == recovered_max);
        assert!(reversed_max == 0xffffffffffffffffffffffffffffffff); // All 1s reversed is still all 1s

        // Test alternating pattern
        let order_id_alt = 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
        let reversed_alt = reverse_bits(order_id_alt);
        let recovered_alt = reverse_bits(reversed_alt);
        assert!(order_id_alt == recovered_alt);
        // 0xaaaa... in binary is 10101010..., reversed should be 01010101... = 0x5555...
        assert!(reversed_alt == 0x55555555555555555555555555555555);


        let order_id_alt = 0x64328946124712951320956108326756;
        let reversed_alt = reverse_bits(order_id_alt);
        let recovered_alt = reverse_bits(reversed_alt);
        assert!(order_id_alt == recovered_alt);
    }
}
