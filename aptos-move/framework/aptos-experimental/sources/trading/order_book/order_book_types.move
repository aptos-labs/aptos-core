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

    const U128_MAX: u128 = 0xffffffffffffffffffffffffffffffff;

    const BIG_MAP_INNER_DEGREE: u16 = 64;
    const BIG_MAP_LEAF_DEGREE: u16 = 32;

    // to replace types:
    struct OrderIdType has store, copy, drop {
        order_id: u128
    }

    /// A type that reverses the bits of OrderIdType to improve parallelism
    /// by distributing orders more evenly across the data structure
    struct ReverseBitsOrderIdType has store, copy, drop {
        reversed_order_id: u128
    }

    struct AccountClientOrderId has store, copy, drop {
        account: address,
        client_order_id: String
    }

    // Internal type representing order in which trades are placed.
    struct UniqueIdxType has store, copy, drop {
        idx: u128
    }

    enum OrderType has store, drop, copy {
        SingleOrder,
        BulkOrder
    }

    public fun single_order_type(): OrderType {
        OrderType::SingleOrder
    }

    public fun bulk_order_type(): OrderType {
        OrderType::BulkOrder
    }

    public fun is_single_order_type(order_type: &OrderType): bool {
        match (order_type) {
            OrderType::SingleOrder => true,
            OrderType::BulkOrder => false,
        }
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

    /// Convert OrderIdType to ReverseBitsOrderIdType by reversing bits
    public(friend) fun to_reverse_bits_order_id(order_id: OrderIdType): ReverseBitsOrderIdType {
        ReverseBitsOrderIdType {
            reversed_order_id: reverse_bits(order_id.order_id)
        }
    }

    /// Convert ReverseBitsOrderIdType back to OrderIdType by reversing bits
    public(friend) fun from_reverse_bits_order_id(reversed_order_id: ReverseBitsOrderIdType): OrderIdType {
        OrderIdType {
            order_id: reverse_bits(reversed_order_id.reversed_order_id)
        }
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
    ): (OrderIdType, address, Option<String>, UniqueIdxType, u64, u64, u64, bool, TimeInForce, M) {
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
            metadata,
        } = self;
        (order_id, account, client_order_id, unique_priority_idx, price, orig_size, remaining_size, is_bid, time_in_force, metadata)
    }

    public(friend) fun destroy_bulk_order_match_details<M: store + copy + drop>(
        self: OrderMatchDetails<M>,
    ): (OrderIdType, address, UniqueIdxType, u64, u64, bool, u64, M) {
        let OrderMatchDetails::BulkOrder {
            order_id,
            account,
            unique_priority_idx,
            price,
            remaining_size,
            is_bid,
            sequence_number,
            metadata,
        } = self;
        (order_id, account, unique_priority_idx, price, remaining_size, is_bid, sequence_number, metadata)
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
            self.client_order_id
        } else {
            option::none()
        }
    }

    public(friend) fun is_bid_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): bool {
        if (self is OrderMatchDetails::SingleOrder) {
            let OrderMatchDetails::SingleOrder { is_bid, .. } = self;
            *is_bid
        } else {
            let OrderMatchDetails::BulkOrder { is_bid, .. } = self;
            *is_bid
        }
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

    public(friend) fun get_sequence_number_from_match_details<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
    ): u64 {
        if (self is OrderMatchDetails::BulkOrder) {
            self.sequence_number
        } else {
            abort 1 // This should only be called on bulk orders
        }
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
            metadata,
        }
    }

    public(friend) fun new_order_match_details_with_modified_size<M: store + copy + drop>(
        self: &OrderMatchDetails<M>,
        remaining_size: u64
    ): OrderMatchDetails<M> {
        if (self is OrderMatchDetails::SingleOrder) {
            let OrderMatchDetails::SingleOrder {
                order_id,
                account,
                client_order_id,
                unique_priority_idx,
                price,
                orig_size,
                remaining_size: _,
                is_bid,
                time_in_force,
                metadata,
            } = self;
            OrderMatchDetails::SingleOrder {
                order_id: *order_id,
                account: *account,
                client_order_id: *client_order_id,
                unique_priority_idx: *unique_priority_idx,
                price: *price,
                orig_size: *orig_size,
                remaining_size,
                is_bid: *is_bid,
                time_in_force: *time_in_force,
                metadata: *metadata,
            }
        } else {
            let OrderMatchDetails::BulkOrder {
                order_id,
                account,
                unique_priority_idx,
                price,
                remaining_size: _,
                is_bid,
                sequence_number,
                metadata,
            } = self;
            OrderMatchDetails::BulkOrder {
                order_id: *order_id,
                account: *account,
                unique_priority_idx: *unique_priority_idx,
                price: *price,
                remaining_size,
                is_bid: *is_bid,
                sequence_number: *sequence_number,
                metadata: *metadata,
            }
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
        (self.order_id, self.matched_size, self.remaining_size, self.order_book_type)
    }

    public(friend) fun get_active_matched_size(self: &ActiveMatchedOrder): u64 {
        self.matched_size
    }

    public(friend) fun get_active_matched_book_type(
        self: &ActiveMatchedOrder
    ): OrderType {
        self.order_book_type
    }

    // ============================= Tests ====================================

    #[test]
    fun test_reverse_bits_order_id_type() {
        // Test basic bit reversal functionality
        let order_id_1 = new_order_id_type(1);
        let order_id_2 = new_order_id_type(2);
        let order_id_3 = new_order_id_type(0x12345678);
        let order_id_4 = new_order_id_type(0x87654321ABCDEF00);

        let reversed_1 = to_reverse_bits_order_id(order_id_1);
        let reversed_2 = to_reverse_bits_order_id(order_id_2);
        let reversed_3 = to_reverse_bits_order_id(order_id_3);
        let reversed_4 = to_reverse_bits_order_id(order_id_4);

        // Test that conversion back gives original value
        let recovered_1 = from_reverse_bits_order_id(reversed_1);
        let recovered_2 = from_reverse_bits_order_id(reversed_2);
        let recovered_3 = from_reverse_bits_order_id(reversed_3);
        let recovered_4 = from_reverse_bits_order_id(reversed_4);

        assert!(order_id_1.get_order_id_value() == recovered_1.get_order_id_value());
        assert!(order_id_2.get_order_id_value() == recovered_2.get_order_id_value());
        assert!(order_id_3.get_order_id_value() == recovered_3.get_order_id_value());
        assert!(order_id_4.get_order_id_value() == recovered_4.get_order_id_value());

        // Test that reversed values are different from originals (for non-palindromic bit patterns)
        // Now we can access the internal field since we're in the same module
        assert!(reversed_1.reversed_order_id != order_id_1.get_order_id_value());
        assert!(reversed_2.reversed_order_id != order_id_2.get_order_id_value());
        assert!(reversed_3.reversed_order_id != order_id_3.get_order_id_value());
        assert!(reversed_4.reversed_order_id != order_id_4.get_order_id_value());

        // Test specific bit reversal cases
        // 1 in binary: 0...0001, reversed should be 1000...0000 (high bit set)
        assert!(reversed_1.reversed_order_id == (1u128 << 127));

        // 2 in binary: 0...0010, reversed should be 0100...0000
        assert!(reversed_2.reversed_order_id == (1u128 << 126));

        // Test edge cases
        let order_id_zero = new_order_id_type(0);
        let reversed_zero = to_reverse_bits_order_id(order_id_zero);
        let recovered_zero = from_reverse_bits_order_id(reversed_zero);
        assert!(order_id_zero.get_order_id_value() == recovered_zero.get_order_id_value());
        assert!(reversed_zero.reversed_order_id == 0); // 0 reversed is still 0

        // Test maximum value
        let order_id_max = new_order_id_type(0xffffffffffffffffffffffffffffffff);
        let reversed_max = to_reverse_bits_order_id(order_id_max);
        let recovered_max = from_reverse_bits_order_id(reversed_max);
        assert!(order_id_max.get_order_id_value() == recovered_max.get_order_id_value());
        assert!(reversed_max.reversed_order_id == 0xffffffffffffffffffffffffffffffff); // All 1s reversed is still all 1s

        // Test alternating pattern
        let order_id_alt = new_order_id_type(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let reversed_alt = to_reverse_bits_order_id(order_id_alt);
        let recovered_alt = from_reverse_bits_order_id(reversed_alt);
        assert!(order_id_alt.get_order_id_value() == recovered_alt.get_order_id_value());
        // 0xaaaa... in binary is 10101010..., reversed should be 01010101... = 0x5555...
        assert!(reversed_alt.reversed_order_id == 0x55555555555555555555555555555555);


        let order_id_alt = new_order_id_type(0x64328946124712951320956108326756);
        let reversed_alt = to_reverse_bits_order_id(order_id_alt);
        let recovered_alt = from_reverse_bits_order_id(reversed_alt);
        assert!(order_id_alt.get_order_id_value() == recovered_alt.get_order_id_value());

    }
}
