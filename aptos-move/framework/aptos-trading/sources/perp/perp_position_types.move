module aptos_trading::perp_position_types {
    use aptos_trading::perp_market_types::AccumulativeIndex;

    enum PerpPosition has store, copy, drop {
        V1 {
            size: u64, // Position size with sz_decimals precision
            entry_px_times_size_sum: u128,
            avg_acquire_entry_px: u64, // Entry price when position was opened
            user_leverage: u8,
            is_long: bool, // true for long positions, false for short positions
            is_isolated: bool, // true for isolated positions, false for cross positions
            funding_index_at_last_update: AccumulativeIndex,
            unrealized_funding_amount_before_last_update: i64,
            timestamp: u64 // timestamp of the last update; for ADL tracking
        }
    }
}
