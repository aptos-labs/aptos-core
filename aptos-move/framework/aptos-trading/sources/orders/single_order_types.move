/// Single Order Types Module
module aptos_trading::single_order_types {
    use std::option::Option;
    use std::string::String;
    use aptos_trading::order_book_types::{
        OrderIdType, IncreasingIdxType,
        TimeInForce, TriggerCondition
    };

    const EORDER_ALREADY_EXISTS: u64 = 1;
    const EINVALID_TRIGGER_CONDITION: u64 = 2;
    const INVALID_MATCH_RESULT: u64 = 3;
    const EINVALID_ORDER_SIZE_DECREASE: u64 = 4;

    enum SingleOrderRequest<M: store + copy + drop> has store, copy, drop {
        V1 {
            account: address,
            order_id: OrderIdType,
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
            unique_priority_idx: IncreasingIdxType,
        }
    }

    enum OrderWithState<M: store + copy + drop> has store, drop, copy {
        V1 {
            order: SingleOrder<M>,
            is_active: bool // i.e. where to find it.
        }
    }

    fun new_order_request_from_match_details<M: store + copy + drop>(
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
            creation_time_micros: option::some(creation_time_micros),
            metadata,
        }
    }

    public(friend) fun new_single_order<M: store + copy + drop>(
        order_request: SingleOrderRequest<M>,
        unique_priority_idx: IncreasingIdxType,
    ): SingleOrder<M> {
        SingleOrder::V1 {
            order_request,
            unique_priority_idx,
        }
    }

    public(friend) fun new_order_with_state<M: store + copy + drop>(
        order: SingleOrder<M>, is_active: bool
    ): OrderWithState<M> {
        OrderWithState::V1 { order, is_active }
    }

    public(friend) fun get_order_id<M: store + copy + drop>(self: &SingleOrderRequest<M>): OrderIdType {
        self.order_id
    }

    public(friend) fun get_account<M: store + copy + drop>(self: &SingleOrderRequest<M>): address {
        self.account
    }

    public(friend) fun get_metadata_from_order<M: store + copy + drop>(self: &SingleOrderRequest<M>): M {
        self.metadata
    }

    public(friend) fun get_time_in_force<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): TimeInForce {
        self.time_in_force
    }

    public(friend) fun get_trigger_condition<M: store + copy + drop>(
        self: &SingleOrderRequest<M>
    ): Option<TriggerCondition> {
        self.trigger_condition
    }

    public(friend) fun get_remaining_size<M: store + copy + drop>(self: &SingleOrderRequest<M>): u64 {
        self.remaining_size
    }
    public(friend) fun get_remaining_size_mut<M: store + copy + drop>(self: &SingleOrderRequest<M>): &mut u64 {
        &mut self.remaining_size
    }

    public(friend) fun get_orig_size<M: store + copy + drop>(self: &SingleOrderRequest<M>): u64 {
        self.orig_size
    }

    public(friend) fun get_client_order_id<M: store + copy + drop>(self: &SingleOrderRequest<M>): Option<String> {
        self.client_order_id
    }

    public(friend) fun get_price<M: store + copy + drop>(self: &SingleOrderRequest<M>): u64 {
        self.price
    }

    public(friend) fun is_bid<M: store + copy + drop>(self: &SingleOrderRequest<M>): bool {
        self.is_bid
    }

    public(friend) fun get_creation_time_micros<M: store + copy + drop>(self: &SingleOrderRequest<M>): u64 {
        self.creation_time_micros
    }

    public(friend) fun get_trigger_condition_mut<M: store + copy + drop>(self: &mut SingleOrderRequest<M>): &mut Option<TriggerCondition> {
        &mut self.trigger_condition
    }

    public(friend) fun get_unique_priority_idx<M: store + copy + drop>(
        self: &SingleOrder<M>
    ): IncreasingIdxType {
        self.unique_priority_idx
    }

    public(friend) fun get_order_request<M: store + copy + drop>(self: &SingleOrder<M>): &SingleOrderRequest<M> {
        &self.order_request
    }

    public(friend) fun get_order_request_mut<M: store + copy + drop>(self: &mut SingleOrder<M>): &mut SingleOrderRequest<M> {
        &mut self.order_request
    }

    public(friend) fun get_order_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): &SingleOrder<M> {
        &self.order
    }

    public(friend) fun get_metadata_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): M {
        self.order.order_request.metadata
    }

    public(friend) fun set_metadata_in_state<M: store + copy + drop>(
        self: &mut OrderWithState<M>, metadata: M
    ) {
        self.order.order_request.metadata = metadata;
    }

    public(friend) fun increase_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        self.order.order_request.remaining_size += size;
    }

    public(friend) fun decrease_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, size: u64
    ) {
        assert!(self.order.order_request.remaining_size > size, EINVALID_ORDER_SIZE_DECREASE);
        self.order.order_request.remaining_size -= size;
    }

    public(friend) fun set_remaining_size<M: store + copy + drop>(
        self: &mut OrderWithState<M>, remaining_size: u64
    ) {
        self.order.order_request.remaining_size = remaining_size;
    }

    public(friend) fun get_remaining_size_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): u64 {
        self.order.order_request.remaining_size
    }

    public(friend) fun get_unique_priority_idx_from_state<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): IncreasingIdxType {
        self.order.unique_priority_idx
    }

    public(friend) fun is_active_order<M: store + copy + drop>(
        self: &OrderWithState<M>
    ): bool {
        self.is_active
    }

    public(friend) fun destroy_order_from_state<M: store + copy + drop>(
        self: OrderWithState<M>
    ): (SingleOrder<M>, bool) {
        let OrderWithState::V1 { order, is_active } = self;
        (order, is_active)
    }

    public fun destroy_single_order<M: store + copy + drop>(
        self: SingleOrder<M>
    ): (SingleOrderRequest<M>, IncreasingIdxType) {
        let SingleOrder::V1 {
            order_request,
            unique_priority_idx,
        } = self;
        (order_request, unique_priority_idx)
    }

    #[test_only]
    public fun new_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
        client_order_id: Option<String>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        creation_time_micros: u64,
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
            creation_time_micros,
            metadata
        }
    }

    #[test_only]
    public fun create_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
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
            creation_time_micros: option::none(),
            metadata
        }
    }

    #[test_only]
    public fun create_simple_test_order_request<M: store + copy + drop>(
        account: address,
        order_id: OrderIdType,
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
        order_id: OrderIdType,
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
