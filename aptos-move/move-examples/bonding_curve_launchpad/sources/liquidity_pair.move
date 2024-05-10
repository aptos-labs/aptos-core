module resource_account::liquidity_pair {
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use resource_account::bonding_curve_launchpad;
	friend bonding_curve_launchpad;


    public(friend) fun test_this( transfer_test: &fungible_asset::TransferRef, swapper_address: address, fa_gained: u64 ){
        primary_fungible_store::transfer_with_ref(transfer_test, @resource_account, swapper_address, fa_gained);
    }
}
