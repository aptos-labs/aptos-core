module aptos_framework::fungible_asset_intent_hooks {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::hot_potato_any::{Self, Any};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object::Object;

    use std::error;

    /// The token offered is not the desired fungible asset.
    const ENOT_DESIRED_TOKEN: u64 = 0;

    /// The token offered does not meet amount requirement.
    const EAMOUNT_NOT_MEET: u64 = 1;

    struct FungibleAssetExchange has store, drop {
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        issuer: address,
    }

    public fun new_fa_to_fa_condition(
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        issuer: address,
    ): FungibleAssetExchange {
        FungibleAssetExchange { desired_metadata, desired_amount, issuer }
    }

    public fun fa_to_fa_consumption(target: Any, argument: Any) {
        let received_fa = hot_potato_any::unpack<FungibleAsset>(target);
        let argument = hot_potato_any::unpack<FungibleAssetExchange>(argument);

        assert!(
            fungible_asset::metadata_from_asset(&received_fa) == argument.desired_metadata,
            error::invalid_argument(ENOT_DESIRED_TOKEN)
        );
        assert!(
            fungible_asset::amount(&received_fa) >= argument.desired_amount,
            error::invalid_argument(EAMOUNT_NOT_MEET),
        );

        primary_fungible_store::deposit(argument.issuer, received_fa);
    }
}
