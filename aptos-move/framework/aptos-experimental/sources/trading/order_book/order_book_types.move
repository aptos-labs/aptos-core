/// (work in progress)
module aptos_experimental::order_book_types {
    use std::option;
    use std::option::Option;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    friend aptos_experimental::active_order_book;
    friend aptos_experimental::order_book;
    friend aptos_experimental::pending_order_book_index;
    friend aptos_experimental::market;

    const U128_MAX: u128 = 0xffffffffffffffffffffffffffffffff;

    const BIG_MAP_INNER_DEGREE: u16 = 64;
    const BIG_MAP_LEAF_DEGREE: u16 = 32;

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EINVALID_TRIGGER_CONDITION: u64 = 2;
    const INVALID_MATCH_RESULT: u64 = 3;
    const EINVALID_ORDER_SIZE_DECREASE: u64 = 4;

    const SLIPPAGE_PCT_PRECISION: u64 = 100; // 100 = 1%

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

    struct ActiveMatchedOrder has copy, drop {
        order_id: OrderIdType,
        matched_size: u64,
        /// Remaining size of the maker order
        remaining_size: u64
    }

    enum SingleOrderMatch<M: store + copy + drop> has drop, copy {
        V1 {
            order: Order<M>,
            matched_size: u64
        }
    }

    enum Order<M: store + copy + drop> has store, copy, drop {
        V1 {
            order_id: OrderIdType,
            account: address,
            client_order_id: Option<u64>, // for client to track orders
            unique_priority_idx: UniqueIdxType,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            trigger_condition: Option<TriggerCondition>,
            metadata: M
        }
    }

    enum TriggerCondition has store, drop, copy {
        TakeProfit(u64),
        StopLoss(u64),
        TimeBased(u64)
    }

    enum OrderWithState<M: store + copy + drop> has store, drop, copy {
        V1 {
            order: Order<M>,
            is_active: bool // i.e. where to find it.
        }
    }

    public(friend) fun new_default_big_ordered_map<K: store, V: store>(): BigOrderedMap<K, V> {
        big_ordered_map::new_with_config(
            BIG_MAP_INNER_DEGREE,
            BIG_MAP_LEAF_DEGREE,
            true
        )
    }

    public fun get_slippage_pct_precision(): u64 {
        SLIPPAGE_PCT_PRECISION
    }

    public fun new_time_based_trigger_condition(time: u64): TriggerCondition {
        TriggerCondition::TimeBased(time)
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

    public(friend) fun new_active_matched_order(
        order_id: OrderIdType, matched_size: u64, remaining_size: u64
    ): ActiveMatchedOrder {
        ActiveMatchedOrder { order_id, matched_size, remaining_size }
    }

    public(friend) fun destroy_active_matched_order(
        self: ActiveMatchedOrder
    ): (OrderIdType, u64, u64) {
        (self.order_id, self.matched_size, self.remaining_size)
    }

    public(friend) fun new_order<M: store + copy + drop>(
        order_id: OrderIdType,
        account: address,
        unique_priority_idx: UniqueIdxType,
        client_order_id: Option<u64>,
        price: u64,
        orig_size: u64,
        size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    ): Order<M> {
        Order::V1 {
            order_id,
            account,
            unique_priority_idx,
            client_order_id,
            price,
            orig_size,
            remaining_size: size,
            is_bid: is_bid,
            trigger_condition,
            metadata
        }
    }

    public(friend) fun new_single_order_match<M: store + copy + drop>(
        order: Order<M>, matched_size: u64
    ): SingleOrderMatch<M> {
        SingleOrderMatch::V1 { order, matched_size }
    }

    public(friend) fun get_active_matched_size(self: &ActiveMatchedOrder): u64 {
        self.matched_size
    }

    public fun get_matched_size<M: store + copy + drop>(
        self: &SingleOrderMatch<M>
    ): u64 {
        self.matched_size
    }

    public fun new_order_with_state<M: store + copy + drop>(
        order: Order<M>, is_active: bool
    ): OrderWithState<M> {
        OrderWithState::V1 { order, is_active }
    }

    public fun tp_trigger_condition(take_profit: u64): TriggerCondition {
        TriggerCondition::TakeProfit(take_profit)
    }

    public fun sl_trigger_condition(stop_loss: u64): TriggerCondition {
        TriggerCondition::StopLoss(stop_loss)
    }

    // Returns the price move down index and price move up index for a particular trigger condition
    public fun index(self: &TriggerCondition, is_bid: bool):
        (Option<u64>, Option<u64>, Option<u64>) {
        match(self) {
            TriggerCondition::TakeProfit(tp) => {
                if (is_bid) {
                    (option::some(*tp), option::none(), option::none())
                } else {
                    (option::none(), option::some(*tp), option::none())
                }
            }
            TriggerCondition::StopLoss(sl) => {
                if (is_bid) {
                    (option::none(), option::some(*sl), option::none())
                } else {
                    (option::some(*sl), option::none(), option::none())
                }
            }
            TriggerCondition::TimeBased(time) => {
                (option::none(), option::none(), option::some(*time))
            }
        }
    }

    public fun get_order_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): &Order<M> {
        &self.order
    }

    public fun get_metadata_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): M {
        self.order.metadata
    }

    public fun get_order_id<M: store + copy + drop>(self: &Order<M>): OrderIdType {
        self.order_id
    }

    public fun get_account<M: store + copy + drop>(self: &Order<M>): address {
        self.account
    }

    public(friend) fun get_unique_priority_idx<M: store + copy + drop>(
        self: &Order<M>
    ): UniqueIdxType {
        self.unique_priority_idx
    }

    public fun get_metadata_from_order<M: store + copy + drop>(self: &Order<M>): M {
        self.metadata
    }

    public fun get_trigger_condition_from_order<M: store + copy + drop>(
        self: &Order<M>
    ): Option<TriggerCondition> {
        self.trigger_condition
    }

    public fun increase_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        self.order.remaining_size += size;
    }

    public fun decrease_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        assert!(self.order.remaining_size > size, EINVALID_ORDER_SIZE_DECREASE);
        self.order.remaining_size -= size;
    }

    public fun set_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, remaining_size: u64
    ) {
        self.order.remaining_size = remaining_size;
    }

    public fun get_remaining_size_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): u64 {
        self.order.remaining_size
    }

    public fun get_unique_priority_idx_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): UniqueIdxType {
        self.order.unique_priority_idx
    }

    public fun get_remaining_size<M: store + copy + drop>(self: &Order<M>): u64 {
        self.remaining_size
    }

    public fun get_orig_size<M: store + copy + drop>(self: &Order<M>): u64 {
        self.orig_size
    }

    public fun get_client_order_id<M: store + copy + drop>(self: &Order<M>): Option<u64> {
        self.client_order_id
    }

    public fun destroy_order_from_state<M: store + copy + drop>(
        self: OrderWithState<M>
    ): (Order<M>, bool) {
        (self.order, self.is_active)
    }

    public fun destroy_active_match_order(self: ActiveMatchedOrder): (OrderIdType, u64, u64) {
        (self.order_id, self.matched_size, self.remaining_size)
    }

    public fun destroy_order<M: store + copy + drop>(
        self: Order<M>
    ): (
        address, OrderIdType, Option<u64>, u64, u64, u64, bool, Option<TriggerCondition>, M
    ) {
        let Order::V1 {
            order_id,
            account,
            client_order_id,
            unique_priority_idx: _,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            metadata
        } = self;
        (
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            metadata
        )
    }

    public fun destroy_single_order_match<M: store + copy + drop>(
        self: SingleOrderMatch<M>
    ): (Order<M>, u64) {
        (self.order, self.matched_size)
    }

    public fun get_order_id_value(self: &OrderIdType): u128 {
        self.order_id
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): bool {
        self.is_active
    }

    public fun get_price<M: store + copy + drop>(self: &Order<M>): u64 {
        self.price
    }

    public fun is_bid<M: store + copy + drop>(self: &Order<M>): bool {
        self.is_bid
    }
}
