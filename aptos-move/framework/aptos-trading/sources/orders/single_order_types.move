/// Single Order Types Module
module aptos_trading::single_order_types {
    use std::option::{Option, Self};
    use std::string::String;
    use aptos_framework::timestamp;
    use aptos_trading::order_book_types::{
        OrderId,
        IncreasingIdx,
        TimeInForce,
        TriggerCondition
    };
    use aptos_trading::order_match_types::OrderMatchDetails;

    #[test_only]
    use aptos_trading::order_book_types::good_till_cancelled;

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EINVALID_TRIGGER_CONDITION: u64 = 2;
    const INVALID_MATCH_RESULT: u64 = 3;
    const EINVALID_ORDER_SIZE_DECREASE: u64 = 4;

    enum SingleOrderRequest<M: store + copy + drop> has store, copy, drop {
        V1 {
            account: address,
            order_id: OrderId,
            client_order_id: Option<String>,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            trigger_condition: Option<TriggerCondition>,
            time_in_force: TimeInForce,
            creation_time_micros: u64,
            metadata: M
        }
    }

    enum SingleOrder<M: store + copy + drop> has store, copy, drop {
        V1 {
            order_request: SingleOrderRequest<M>,
            unique_priority_idx: IncreasingIdx
        }
    }

    enum OrderWithState<M: store + copy + drop> has store, drop, copy {
        V1 {
            order: SingleOrder<M>,
            is_active: bool // i.e. where to find it.
        }
    }

    public fun new_order_request_from_match_details<M: store + copy + drop>(
        order_match_details: OrderMatchDetails<M>
    ): SingleOrderRequest<M> {
        let (
            order_id,
            account,
            client_order_id,
            _unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            time_in_force,
            creation_time_micros,
            metadata
        ) = order_match_details.destroy_single_order_match_details();
        SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition: option::none(),
            time_in_force,
            creation_time_micros,
            metadata
        }
    }

    public fun new_single_order<M: store + copy + drop>(
        order_request: SingleOrderRequest<M>, unique_priority_idx: IncreasingIdx
    ): SingleOrder<M> {
        SingleOrder::V1 { order_request, unique_priority_idx }
    }

    public fun new_order_with_state<M: store + copy + drop>(
        order: SingleOrder<M>, is_active: bool
    ): OrderWithState<M> {
        OrderWithState::V1 { order, is_active }
    }

    public fun get_order_id<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): OrderId {
        self.order_id
    }

    public fun get_account<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): address {
        self.account
    }

    // public fun get_metadata<M: store + copy + drop>(
    //     self: &SingleOrderRequest<M>
    // ): M {
    //     self.metadata
    // }

    // public fun get_time_in_force<M: store + copy + drop>(
    //     self: &SingleOrderRequest<M>
    // ): TimeInForce {
    //     self.time_in_force
    // }

    public fun get_trigger_condition<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): Option<TriggerCondition> {
        self.trigger_condition
    }

    #[test_only]
    public fun get_trigger_condition_mut<M: store + copy + drop>(
        self: &mut SingleOrderRequest<M>
    ): &mut Option<TriggerCondition> {
        &mut self.trigger_condition
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): u64 {
        self.remaining_size
    }

    #[test_only]
    public fun get_remaining_size_mut<M: store + copy + drop>(
        self: &mut SingleOrderRequest<M>
    ): &mut u64 {
        &mut self.remaining_size
    }

    // public(friend) fun get_orig_size<M: store + copy + drop>(
    //     self: &SingleOrderRequest<M>
    // ): u64 {
    //     self.orig_size
    // }

    public fun get_client_order_id<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): Option<String> {
        self.client_order_id
    }

    public fun get_price<M: store + copy + drop>(self: &SingleOrderRequest<M>): u64 {
        self.price
    }

    public fun is_bid<M: store + copy + drop>(self: &SingleOrderRequest<M>): bool {
        self.is_bid
    }

    public fun get_creation_time_micros<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): u64 {
        self.creation_time_micros
    }

    public fun get_unique_priority_idx<M: store + copy + drop>(
        self: &SingleOrder<M>
    ): IncreasingIdx {
        self.unique_priority_idx
    }

    public fun get_order_request<M: store + copy + drop>(
        self: &SingleOrder<M>
    ): &SingleOrderRequest<M> {
        &self.order_request
    }

    public fun get_order_request_mut<M: store + copy + drop>(
        self: &mut SingleOrder<M>
    ): &mut SingleOrderRequest<M> {
        &mut self.order_request
    }

    public fun get_order_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): &SingleOrder<M> {
        &self.order
    }

    public fun get_order_from_state_mut<M: store + copy + drop>(
        self: &mut OrderWithState<M>
    ): &mut SingleOrder<M> {
        &mut self.order
    }

    public fun get_metadata_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): M {
        self.order.order_request.metadata
    }

    public fun set_metadata_in_state<M: store + copy + drop>(
        self: &mut OrderWithState<M>, metadata: M
    ) {
        self.order.order_request.metadata = metadata;
    }

    public fun increase_remaining_size_from_state<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        self.order.order_request.remaining_size += size;
    }

    public fun decrease_remaining_size_from_state<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        assert!(
            self.order.order_request.remaining_size > size,
            EINVALID_ORDER_SIZE_DECREASE
        );
        self.order.order_request.remaining_size -= size;
    }

    public fun set_remaining_size_from_state<M: store + copy + drop>(
        self: &mut OrderWithState<M>, remaining_size: u64
    ) {
        self.order.order_request.remaining_size = remaining_size;
    }

    public fun get_remaining_size_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): u64 {
        self.order.order_request.remaining_size
    }

    public fun get_unique_priority_idx_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): IncreasingIdx {
        self.order.unique_priority_idx
    }

    public fun is_active_order<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): bool {
        self.is_active
    }

    public fun destroy_order_from_state<M: store + copy + drop>(
        self: OrderWithState<M>
    ): (SingleOrder<M>, bool) {
        let OrderWithState::V1 { order, is_active } = self;
        (order, is_active)
    }

    public fun destroy_single_order<M: store + copy + drop>(
        self: SingleOrder<M>
    ): (SingleOrderRequest<M>, IncreasingIdx) {
        let SingleOrder::V1 { order_request, unique_priority_idx } = self;
        (order_request, unique_priority_idx)
    }

    public fun destroy_single_order_request<M: store + copy + drop>(
        self: SingleOrderRequest<M>
    ): (
        address,
        OrderId,
        Option<String>,
        u64,
        u64,
        u64,
        bool,
        Option<TriggerCondition>,
        TimeInForce,
        u64,
        M
    ) {
        let SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            metadata,
            creation_time_micros
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
            time_in_force,
            creation_time_micros,
            metadata
        )
    }

    public fun new_single_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        client_order_id: Option<String>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        metadata: M
    ): SingleOrderRequest<M> {
        SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            creation_time_micros: timestamp::now_microseconds(),
            metadata
        }
    }

    #[test_only]
    public fun create_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        client_order_id: Option<String>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        metadata: M
    ): SingleOrderRequest<M> {
        SingleOrderRequest::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force: good_till_cancelled(),
            creation_time_micros: timestamp::now_microseconds(),
            metadata
        }
    }

    #[test_only]
    public fun create_simple_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        price: u64,
        size: u64,
        is_bid: bool,
        metadata: M
    ): SingleOrderRequest<M> {
        create_test_order_request(
            account,
            order_id,
            option::none(),
            price,
            size,
            size,
            is_bid,
            option::none(),
            metadata
        )
    }

    #[test_only]
    public fun create_test_order_request_with_client_id<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        client_order_id: String,
        price: u64,
        size: u64,
        is_bid: bool,
        metadata: M
    ): SingleOrderRequest<M> {
        create_test_order_request(
            account,
            order_id,
            option::some(client_order_id),
            price,
            size,
            size,
            is_bid,
            option::none(),
            metadata
        )
    }
}
