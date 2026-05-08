module aptos_trading::perp_state_store {
    use aptos_trading::native_store_capability::NativeStoreCapability;
    use aptos_trading::perp_market_price_types::PerpMarketState;
    use aptos_trading::perp_position_types::PerpPosition;
    use aptos_trading::perp_collateral_types::PerpCollateralBalance;

    native fun set_market_state(capability: &NativeStoreCapability, market: address, state: PerpMarketState);

    native fun set_position(capability: &NativeStoreCapability, market: address, account: address, position: PerpPosition);
    native fun delete_position(capability: &NativeStoreCapability, market: address, account: address);

    native fun set_cross_collateral_balance(capability: &NativeStoreCapability, account: address, balance: PerpCollateralBalance);
    native fun set_isolated_collateral_balance(capability: &NativeStoreCapability, account: address, market: address, balance: PerpCollateralBalance);

    native fun set_collateral_asset_value(capability: &NativeStoreCapability, account: address, asset_type: address, value: u64);


    // we might need some global constants, not sure.
}
