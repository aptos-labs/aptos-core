spec aptos_trading::order_book_types {

    spec bulk_order_type(): 0x5::order_book_types::OrderType {
        pragma opaque = true;
        ensures [inferred] result == OrderType { type: 1 };
        aborts_if [inferred] false;
    }

    spec get_order_id_value(self: &0x5::order_book_types::OrderId): u128 {
        pragma opaque = true;
        ensures [inferred] result == self.order_id;
        aborts_if [inferred] false;
    }

    spec get_trigger_condition_indices(
        self: &0x5::order_book_types::TriggerCondition
    ): (
        0x1::option::Option<u64>, 0x1::option::Option<u64>, 0x1::option::Option<u64>
    ) {
        use 0x1::option;
        ensures (self is PriceMoveAbove) ==>
            result_1 == option::none<u64>()
                && result_2 == option::some<u64>(self.0)
                && result_3 == option::none<u64>();
        ensures (self is PriceMoveBelow) ==>
            result_1 == option::some<u64>(self.0)
                && result_2 == option::none<u64>()
                && result_3 == option::none<u64>();
        ensures (self is TimeBased) ==>
            result_1 == option::none<u64>()
                && result_2 == option::none<u64>()
                && result_3 == option::some<u64>(self.0);
        aborts_if false;
    }

    spec good_till_cancelled(): 0x5::order_book_types::TimeInForce {
        pragma opaque = true;
        ensures [inferred] result == TimeInForce::GTC {};
        aborts_if [inferred] false;
    }

    spec immediate_or_cancel(): 0x5::order_book_types::TimeInForce {
        pragma opaque = true;
        ensures [inferred] result == TimeInForce::IOC {};
        aborts_if [inferred] false;
    }

    spec into_decreasing_idx_type(
        self: &0x5::order_book_types::IncreasingIdx
    ): 0x5::order_book_types::DecreasingIdx {
        pragma opaque = true;
        ensures [inferred] result == DecreasingIdx { idx: MAX_U128 - self.idx };
        aborts_if [inferred] MAX_U128 - self.idx < 0;
    }

    spec is_bulk_order_type(
        order_type: &0x5::order_book_types::OrderType
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (order_type.type == 1);
        aborts_if [inferred] false;
    }

    spec is_single_order_type(
        order_type: &0x5::order_book_types::OrderType
    ): bool {
        pragma opaque = true;
        ensures [inferred] result == (order_type.type == 0);
        aborts_if [inferred] false;
    }

    spec new_account_client_order_id(
        account: address, client_order_id: 0x1::string::String
    ): 0x5::order_book_types::AccountClientOrderId {
        pragma opaque = true;
        ensures [inferred] result
            == AccountClientOrderId {
                account: account,
                client_order_id: client_order_id
            };
        aborts_if [inferred] false;
    }

    spec new_order_id_type(order_id: u128): 0x5::order_book_types::OrderId {
        pragma opaque = true;
        ensures [inferred] result == OrderId { order_id: order_id };
        aborts_if [inferred] false;
    }

    spec new_time_based_trigger_condition(time_secs: u64)
        : 0x5::order_book_types::TriggerCondition {
        pragma opaque = true;
        ensures [inferred] result == TriggerCondition::TimeBased(time_secs);
        aborts_if [inferred] false;
    }

    spec next_increasing_idx_type(): 0x5::order_book_types::IncreasingIdx {
        pragma opaque = true;
        aborts_if [inferred] false;
    }

    spec post_only(): 0x5::order_book_types::TimeInForce {
        pragma opaque = true;
        ensures [inferred] result == TimeInForce::POST_ONLY {};
        aborts_if [inferred] false;
    }

    spec price_move_down_condition(price: u64): 0x5::order_book_types::TriggerCondition {
        pragma opaque = true;
        ensures [inferred] result == TriggerCondition::PriceMoveBelow(price);
        aborts_if [inferred] false;
    }

    spec price_move_up_condition(price: u64): 0x5::order_book_types::TriggerCondition {
        pragma opaque = true;
        ensures [inferred] result == TriggerCondition::PriceMoveAbove(price);
        aborts_if [inferred] false;
    }

    spec single_order_type(): 0x5::order_book_types::OrderType {
        pragma opaque = true;
        ensures [inferred] result == OrderType { type: 0 };
        aborts_if [inferred] false;
    }

    spec time_in_force_from_index(index: u8): 0x5::order_book_types::TimeInForce {
        pragma opaque = true;
        ensures [inferred] index == 0 ==> result == TimeInForce::GTC {};
        ensures [inferred] index != 0 && index == 1 ==>
            result == TimeInForce::POST_ONLY {};
        ensures [inferred] index != 0
            && (index != 1
                && index == 2) ==>
            result == TimeInForce::IOC {};
        aborts_if [inferred] index != 0 && (index != 1 && index != 2);
    }
}
