spec aptos_trading::single_order_types {

    spec is_bid<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == self.is_bid;
        aborts_if [inferred] false;
    }

    spec get_account<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): address {
        pragma opaque = true;
        ensures [inferred] result == self.account;
        aborts_if [inferred] false;
    }

    spec get_creation_time_micros<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.creation_time_micros;
        aborts_if [inferred] false;
    }

    spec get_order_id<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): 0x5::order_book_types::OrderId {
        pragma opaque = true;
        ensures [inferred] result == self.order_id;
        aborts_if [inferred] false;
    }

    spec get_order_request<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrder<M>
    ): &0x5::single_order_types::SingleOrderRequest<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order_request;
        aborts_if [inferred] false;
    }

    spec get_order_request_mut<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::SingleOrder<M>
    ): &mut 0x5::single_order_types::SingleOrderRequest<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order_request;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec get_unique_priority_idx<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrder<M>
    ): 0x5::order_book_types::IncreasingIdx {
        pragma opaque = true;
        ensures [inferred] result == self.unique_priority_idx;
        aborts_if [inferred] false;
    }

    /// Exact local effect of the source assertion and subtraction; it has no
    /// global frame.
    spec decrease_remaining_size_from_state<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::OrderWithState<M>, size: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if self.order.order_request.remaining_size <= size;
        ensures self
            == update_field(
                old(self),
                order,
                update_field(
                    old(self).order,
                    order_request,
                    update_field(
                        old(self).order.order_request,
                        remaining_size,
                        old(self).order.order_request.remaining_size - size
                    )
                )
            );
    }

    spec destroy_order_from_state<M: copy + drop + store>(
        self: 0x5::single_order_types::OrderWithState<M>
    ): (0x5::single_order_types::SingleOrder<M>, bool) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order;
        ensures [inferred] result_2 == self.is_active;
        aborts_if [inferred] false;
    }

    spec destroy_single_order<M: copy + drop + store>(
        self: 0x5::single_order_types::SingleOrder<M>
    ): (
        0x5::single_order_types::SingleOrderRequest<M>,
        0x5::order_book_types::IncreasingIdx
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.order_request;
        ensures [inferred] result_2 == self.unique_priority_idx;
        aborts_if [inferred] false;
    }

    spec destroy_single_order_request<M: copy + drop + store>(
        self: 0x5::single_order_types::SingleOrderRequest<M>
    ): (
        address,
        0x5::order_book_types::OrderId,
        0x1::option::Option<0x1::string::String>,
        u64,
        u64,
        u64,
        bool,
        0x1::option::Option<0x5::order_book_types::TriggerCondition>,
        0x5::order_book_types::TimeInForce,
        u64,
        M
    ) {
        pragma opaque = true;
        ensures [inferred] result_1 == self.account;
        ensures [inferred] result_2 == self.order_id;
        ensures [inferred] result_3 == self.client_order_id;
        ensures [inferred] result_4 == self.price;
        ensures [inferred] result_5 == self.orig_size;
        ensures [inferred] result_6 == self.remaining_size;
        ensures [inferred] result_7 == self.is_bid;
        ensures [inferred] result_8 == self.trigger_condition;
        ensures [inferred] result_9 == self.time_in_force;
        ensures [inferred] result_10 == self.creation_time_micros;
        ensures [inferred] result_11 == self.metadata;
        aborts_if [inferred] false;
    }

    spec get_client_order_id<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): 0x1::option::Option<0x1::string::String> {
        pragma opaque = true;
        ensures [inferred] result == self.client_order_id;
        aborts_if [inferred] false;
    }

    spec get_metadata_from_state<M: copy + drop + store>(
        self: &0x5::single_order_types::OrderWithState<M>
    ): M {
        pragma opaque = true;
        ensures [inferred] result == self.order.order_request.metadata;
        aborts_if [inferred] false;
    }

    spec get_order_from_state<M: copy + drop + store>(
        self: &0x5::single_order_types::OrderWithState<M>
    ): &0x5::single_order_types::SingleOrder<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order;
        aborts_if [inferred] false;
    }

    spec get_order_from_state_mut<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::OrderWithState<M>
    ): &mut 0x5::single_order_types::SingleOrder<M> {
        pragma opaque = true;
        ensures [inferred] result == self.order;
        ensures [inferred] self == old(self);
        aborts_if [inferred] false;
    }

    spec get_price<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.price;
        aborts_if [inferred] false;
    }

    spec get_remaining_size<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.remaining_size;
        aborts_if [inferred] false;
    }

    spec get_remaining_size_from_state<M: copy + drop + store>(
        self: &0x5::single_order_types::OrderWithState<M>
    ): u64 {
        pragma opaque = true;
        ensures [inferred] result == self.order.order_request.remaining_size;
        aborts_if [inferred] false;
    }

    spec get_trigger_condition<M: copy + drop + store>(
        self: &0x5::single_order_types::SingleOrderRequest<M>
    ): 0x1::option::Option<0x5::order_book_types::TriggerCondition> {
        pragma opaque = true;
        ensures [inferred] result == self.trigger_condition;
        aborts_if [inferred] false;
    }

    spec get_unique_priority_idx_from_state<M: copy + drop + store>(
        self: &0x5::single_order_types::OrderWithState<M>
    ): 0x5::order_book_types::IncreasingIdx {
        pragma opaque = true;
        ensures [inferred] result == self.order.unique_priority_idx;
        aborts_if [inferred] false;
    }

    /// Exact local effect of the source addition; it has no global frame.
    spec increase_remaining_size_from_state<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::OrderWithState<M>, size: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if self.order.order_request.remaining_size + size > MAX_U64;
        ensures self
            == update_field(
                old(self),
                order,
                update_field(
                    old(self).order,
                    order_request,
                    update_field(
                        old(self).order.order_request,
                        remaining_size,
                        old(self).order.order_request.remaining_size + size
                    )
                )
            );
    }

    spec is_active_order<M: copy + drop + store>(
        self: &0x5::single_order_types::OrderWithState<M>
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == self.is_active;
        aborts_if [inferred] false;
    }

    spec new_order_request_from_match_details<M: copy + drop + store>(
        order_match_details: 0x5::order_match_types::OrderMatchDetails<M>
    ): 0x5::single_order_types::SingleOrderRequest<M> {
        use 0x1::option;
        use 0x5::order_book_types;
        use 0x5::order_match_types;
        pragma opaque = true;
        ensures [inferred]({
            let (
                a_0, a_1, a_2, a_3, a_4, a_5, a_6, a_7, a_8, a_9, a_10
            ) =
                result_of<order_match_types::destroy_single_order_match_details<M>> (
                    order_match_details
                );
            result
                == SingleOrderRequest::V1<M> {
                    account: a_1,
                    order_id: a_0,
                    client_order_id: a_2,
                    price: a_4,
                    orig_size: a_5,
                    remaining_size: a_6,
                    is_bid: a_7,
                    trigger_condition: option::none<order_book_types::TriggerCondition>(),
                    time_in_force: a_8,
                    creation_time_micros: a_9,
                    metadata: a_10
                }
        });
        aborts_if [inferred] aborts_of<order_match_types::destroy_single_order_match_details<M
            >> (order_match_details);
    }

    spec new_order_with_state<M: copy + drop + store>(
        order: 0x5::single_order_types::SingleOrder<M>, is_active: bool
    ): 0x5::single_order_types::OrderWithState<M> {
        pragma opaque = true;
        ensures [inferred] result
            == OrderWithState::V1<M> { order: order, is_active: is_active };
        aborts_if [inferred] false;
    }

    spec new_single_order<M: copy + drop + store>(
        order_request: 0x5::single_order_types::SingleOrderRequest<M>,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx
    ): 0x5::single_order_types::SingleOrder<M> {
        pragma opaque = true;
        ensures [inferred] result
            == SingleOrder::V1<M> {
                order_request: order_request,
                unique_priority_idx: unique_priority_idx
            };
        aborts_if [inferred] false;
    }

    spec set_metadata_in_state<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::OrderWithState<M>, metadata: M
    ) {
        pragma opaque = true;
        ensures [inferred] self
            == update_field(
                old(self),
                order,
                update_field(
                    old(self).order,
                    order_request,
                    update_field(old(self).order.order_request, metadata, metadata)
                )
            );
        aborts_if [inferred] false;
    }

    /// Exact local assignment in the source; it has no global frame.
    spec set_remaining_size_from_state<M: copy + drop + store>(
        self: &mut 0x5::single_order_types::OrderWithState<M>, remaining_size: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = false;
        aborts_if false;
        ensures self
            == update_field(
                old(self),
                order,
                update_field(
                    old(self).order,
                    order_request,
                    update_field(
                        old(self).order.order_request, remaining_size, remaining_size
                    )
                )
            );
    }
}
