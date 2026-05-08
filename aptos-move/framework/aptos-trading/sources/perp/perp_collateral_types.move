module aptos_trading::perp_collateral_types {
    enum AssetBalance has store, drop, copy {
        V1 {
            asset_type: address,
            balance: u64
        }
    }

    enum PerpCollateralBalance has store, copy, drop {
        V1 {
            primary_balance: u64,
            secondary_balances: vector<AssetBalance>,
        }
    }
}
