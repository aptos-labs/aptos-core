spec aptos_experimental::native_position_types {
    spec new_accumulative_index(index: i128): AccumulativeIndex {
        pragma opaque;
        aborts_if false;
        ensures result == AccumulativeIndex { index };
    }

    spec accumulative_index_value(idx: &AccumulativeIndex): i128 {
        pragma opaque;
        aborts_if false;
        ensures result == idx.index;
    }

    spec new_perp_v1(
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_acquire_entry_px: u64,
        user_leverage: u8,
        is_isolated: bool,
        funding_index_at_last_update: AccumulativeIndex,
        unrealized_funding_amount_before_last_update: i64,
        timestamp: u64,
    ): Position {
        pragma opaque;
        aborts_if false;
        ensures result == Position::PerpV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_acquire_entry_px,
            user_leverage,
            is_isolated,
            funding_index_at_last_update,
            unrealized_funding_amount_before_last_update,
            timestamp,
        };
    }

    spec unpack_perp_v1(pos: Position): (u64, bool, u128, u64, u8, bool, AccumulativeIndex, i64, u64) {
        pragma opaque;
        aborts_if false;
        ensures result_1 == pos.size;
        ensures result_2 == pos.is_long;
        ensures result_3 == pos.entry_px_times_size_sum;
        ensures result_4 == pos.avg_acquire_entry_px;
        ensures result_5 == pos.user_leverage;
        ensures result_6 == pos.is_isolated;
        ensures result_7 == pos.funding_index_at_last_update;
        ensures result_8 == pos.unrealized_funding_amount_before_last_update;
        ensures result_9 == pos.timestamp;
    }
}
