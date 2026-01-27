/// Order book type definitions
module aptos_trading::order_book_types {
    friend aptos_trading::bulk_order_types;
    friend aptos_trading::single_order_types;

    use std::option;
    use std::string::String;
    use aptos_framework::transaction_context;

    const U128_MAX: u128 = 0xffffffffffffffffffffffffffffffff;

    const SINGLE_ORDER_TYPE: u16 = 0;
    const BULK_ORDER_TYPE: u16 = 1;

    const EINVALID_TIME_IN_FORCE: u64 = 5;

    struct OrderId has store, copy, drop {
        order_id: u128
    }

    struct AccountClientOrderId has store, copy, drop {
        account: address,
        client_order_id: String
    }

    // Internal type representing order in which trades are placed.
    struct IncreasingIdx has store, copy, drop {
        idx: u128
    }

    struct DecreasingIdx has store, copy, drop {
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

    public fun next_order_id(): OrderId {
        // reverse bits to make order ids random, so indices on top of them are shuffled.
        OrderId {
            order_id: reverse_bits(
                transaction_context::monotonically_increasing_counter()
            )
        }
    }

    public fun new_order_id_type(order_id: u128): OrderId {
        OrderId { order_id }
    }

    public fun new_account_client_order_id(
        account: address, client_order_id: String
    ): AccountClientOrderId {
        AccountClientOrderId { account, client_order_id }
    }

    public fun next_increasing_idx_type(): IncreasingIdx {
        IncreasingIdx { idx: transaction_context::monotonically_increasing_counter() }
    }

    #[test_only]
    public fun new_increasing_idx_type(idx: u128): IncreasingIdx {
        IncreasingIdx { idx }
    }

    public fun into_decreasing_idx_type(self: &IncreasingIdx): DecreasingIdx {
        DecreasingIdx { idx: U128_MAX - self.idx }
    }

    public fun get_order_id_value(self: &OrderId): u128 {
        self.order_id
    }

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
        match(self) { GTC => 0, POST_ONLY => 1, IOC => 2 }
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
    public fun get_trigger_condition_indices(
        self: &TriggerCondition
    ): (option::Option<u64>, option::Option<u64>, option::Option<u64>) {
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

    /// Reverse the bits in a u128 value using divide and conquer approach
    /// This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)
    fun reverse_bits(value: u128): u128 {
        let v = value;

        // Swap odd and even bits
        v =
            ((v & 0x55555555555555555555555555555555) << 1)
                | ((v >> 1) & 0x55555555555555555555555555555555);

        // Swap consecutive pairs
        v =
            ((v & 0x33333333333333333333333333333333) << 2)
                | ((v >> 2) & 0x33333333333333333333333333333333);

        // Swap nibbles
        v =
            ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) << 4)
                | ((v >> 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

        // Swap bytes
        v =
            ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8)
                | ((v >> 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

        // Swap 2-byte chunks
        v =
            ((v & 0x0000ffff0000ffff0000ffff0000ffff) << 16)
                | ((v >> 16) & 0x0000ffff0000ffff0000ffff0000ffff);

        // Swap 4-byte chunks
        v =
            ((v & 0x00000000ffffffff00000000ffffffff) << 32)
                | ((v >> 32) & 0x00000000ffffffff00000000ffffffff);

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

    #[test_only]
    struct TestMetadata has store, copy, drop {}

    #[test_only]
    public fun new_test_metadata(): TestMetadata {
        TestMetadata {}
    }
}
