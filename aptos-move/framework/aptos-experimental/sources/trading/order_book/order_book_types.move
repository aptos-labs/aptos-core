/// Order book type definitions
module aptos_experimental::order_book_types {
    use std::option;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    friend aptos_experimental::price_time_index;
    friend aptos_experimental::single_order_book;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::market;
    friend aptos_experimental::order_book;
    friend aptos_experimental::single_order_types;

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

    // Internal type representing order in which trades are placed. Unique per instance of AscendingIdGenerator.
    struct UniqueIdxType has store, copy, drop {
        idx: u128
    }

    // Struct providing ascending ids, to be able to be used as tie-breaker to respect FIFO order of trades.
    // Returned ids are ascending and unique within a single instance of AscendingIdGenerator.
    enum AscendingIdGenerator has store, drop {
        FromCounter {
            value: u64
        }
        // TODO: add stateless (and with that fully parallel) support for id creation via native function
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

    public(friend) fun new_ascending_id_generator(): AscendingIdGenerator {
        AscendingIdGenerator::FromCounter { value: 0 }
    }

    public(friend) fun next_ascending_id(self: &mut AscendingIdGenerator): u128 {
        self.value += 1;
        self.value as u128
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
}
