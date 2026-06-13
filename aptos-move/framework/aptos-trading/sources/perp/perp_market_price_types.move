module aptos_trading::perp_market_price_types {
    struct AccumulativeIndex has store, copy, drop {
        index: i128
    }

    enum PerpMarketPriceConfig has store, drop {
        V1 {
            size_multiplier: u64,
            unrealized_pnl_haircut_bps: u64,
            max_leverage: u8
        }
    }

    enum PerpMarketPriceState has store, drop {
        V1 {
            /// largest mark price in the mark_prices vector
            short_mark_px: u64,
            /// smallest mark price in the mark_prices vector
            long_mark_px: u64,
            accumulative_index: AccumulativeIndex
        }
    }

    enum PerpMarketState has drop {
        V1 {
            config: PerpMarketPriceConfig,
            price_state: PerpMarketPriceState,
        }
    }
}
