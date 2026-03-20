/// Order book type definitions
module aptos_trading::order_book_types {
    friend aptos_trading::bulk_order_types;
    friend aptos_trading::single_order_types;

    use std::option;
    use std::string::String;
    use aptos_framework::transaction_context;

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
        DecreasingIdx { idx: MAX_U128 - self.idx }
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

    #[test_only]
    struct TestMetadata has store, copy, drop {}

    #[test_only]
    public fun new_test_metadata(): TestMetadata {
        TestMetadata {}
    }
}
