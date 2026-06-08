/// Types crossing the `native_position` Move ↔ native boundary. Field
/// order/widths here pin the BCS encoding the Rust `NativePosition`
/// must match.
module aptos_trading::native_position_types {
    /// Signed accumulative index (funding, premium, ...). Wrapping a raw
    /// `i128` avoids passing the wrong value where an index is expected.
    struct AccumulativeIndex has copy, drop, store {
        index: i128,
    }

    public fun new_accumulative_index(index: i128): AccumulativeIndex {
        AccumulativeIndex { index }
    }

    public fun accumulative_index_value(idx: &AccumulativeIndex): i128 {
        idx.index
    }

    /// Mirrors the Rust `NativePosition`. All numeric fields are at
    /// producer-defined precision; the chain is precision-agnostic.
    enum Position has copy, drop, store {
        PerpV1 {
            size: u64,
            is_long: bool,
            entry_px_times_size_sum: u128,
            avg_acquire_entry_px: u64,
            user_leverage: u8,
            is_isolated: bool,
            funding_index_at_last_update: AccumulativeIndex,
            unrealized_funding_amount_before_last_update: i64,
            // Microsecond timestamp of the last update; used for ADL.
            timestamp: u64,
        },
    }

    public fun new_perp_v1(
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
        Position::PerpV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_acquire_entry_px,
            user_leverage,
            is_isolated,
            funding_index_at_last_update,
            unrealized_funding_amount_before_last_update,
            timestamp,
        }
    }

    public fun unpack_perp_v1(
        pos: Position,
    ): (u64, bool, u128, u64, u8, bool, AccumulativeIndex, i64, u64) {
        let Position::PerpV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_acquire_entry_px,
            user_leverage,
            is_isolated,
            funding_index_at_last_update,
            unrealized_funding_amount_before_last_update,
            timestamp,
        } = pos;
        (
            size,
            is_long,
            entry_px_times_size_sum,
            avg_acquire_entry_px,
            user_leverage,
            is_isolated,
            funding_index_at_last_update,
            unrealized_funding_amount_before_last_update,
            timestamp,
        )
    }
}
