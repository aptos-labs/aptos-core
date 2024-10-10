module aptos_framework::fungible_asset_intent {
    use std::string;
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata, FungibleStore};
    use aptos_framework::fungible_asset_intent_hooks::{Self, FungibleAssetExchange};
    use aptos_framework::intent::{Self, TradeSession, TradeIntent};
    use aptos_framework::object::{Self, DeleteRef, ExtendRef, Object};

    struct FungibleStoreManager has store {
        extend_ref: ExtendRef,
        delete_ref: DeleteRef,
    }

    public fun create_fa_to_fa_intent(
        source_fungible_asset: FungibleAsset,
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        expiry_time: u64,
        issuer: address,
    ): Object<TradeIntent<FungibleStoreManager, FungibleAsset, FungibleAssetExchange>> {
        let coin_store_ref = object::create_self_owned_object();
        let extend_ref = object::generate_extend_ref(&coin_store_ref);
        let delete_ref = object::generate_delete_ref(&coin_store_ref);
        fungible_asset::create_store(&coin_store_ref, fungible_asset::metadata_from_asset(&source_fungible_asset));
        fungible_asset::deposit(
            object::object_from_constructor_ref<FungibleStore>(&coin_store_ref),
            source_fungible_asset
        );
        let dispatch_function_info = function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b"fungible_asset_intent_hooks"),
            string::utf8(b"fa_to_fa_consumption"),
        );
        intent::create_intent(
            FungibleStoreManager { extend_ref, delete_ref},
            fungible_asset_intent_hooks::new_fa_to_fa_condition(desired_metadata, desired_amount, issuer),
            expiry_time,
            dispatch_function_info,
            issuer,
        )
    }

    public fun start_fa_to_fa_session(
        intent: Object<TradeIntent<FungibleStoreManager, FungibleAsset, FungibleAssetExchange>>
    ): (FungibleAsset, TradeSession<FungibleAsset, FungibleAssetExchange>) {
        let (store_manager, session) = intent::start_intent_session(intent);
        let FungibleStoreManager { extend_ref, delete_ref } = store_manager;
        let store_signer = object::generate_signer_for_extending(&extend_ref);
        let fa_store = object::object_from_delete_ref<FungibleStore>(&delete_ref);
        let fa = fungible_asset::withdraw(&store_signer, fa_store, fungible_asset::balance(fa_store));
        fungible_asset::remove_store(&delete_ref);
        object::delete(delete_ref);
        (fa, session)
    }

    public fun finish_fa_to_fa_session(
        session: TradeSession<FungibleAsset, FungibleAssetExchange>,
        desired_token: FungibleAsset,
    ) {
        intent::finish_intent_session(session, desired_token)
    }

    #[test(
        aptos_framework = @0x1,
        creator1 = @0xcafe,
        creator2 = @0xcaff,
        aaron = @0xface,
        offerer = @0xbadd
    )]
    fun test_e2e_basic_flow(
        aptos_framework: &signer,
        creator1: &signer,
        creator2: &signer,
        aaron: &signer,
    ) {
        use aptos_framework::timestamp;
        use aptos_framework::signer;
        use aptos_framework::primary_fungible_store;

        timestamp::set_time_has_started_for_testing(aptos_framework);
        let (mint_ref_1, _, burn_ref_1, _, test_token_1) = fungible_asset::create_fungible_asset(creator1);
        let (creator_ref, metadata) = fungible_asset::create_test_token(creator2);
        primary_fungible_store::init_test_metadata_with_primary_store_enabled(&creator_ref);
        let mint_ref_2 = fungible_asset::generate_mint_ref(&creator_ref);
        let test_token_2 = object::convert(metadata);

        let fa1 = fungible_asset::mint(&mint_ref_1, 10);
        // Register intent to trade 10 FA1 into 5 FA2.
        let intent = create_fa_to_fa_intent(
            fa1,
            test_token_2,
            5,
            1,
            signer::address_of(aaron),
        );

        let (fa1, session) = start_fa_to_fa_session(intent);

        assert!(fungible_asset::metadata_from_asset(&fa1) == test_token_1, 1);
        assert!(fungible_asset::amount(&fa1) == 10, 1);

        // Mint FA2 for the sake of testing. In the real life we expect a DeFi app to perform the trade.
        let fa2 = fungible_asset::mint(&mint_ref_2, 5);
        fungible_asset::burn(&burn_ref_1, fa1);

        // Trade FA1 for 5 FA2
        finish_fa_to_fa_session(session, fa2);

        assert!(primary_fungible_store::balance(signer::address_of(aaron), test_token_2) == 5, 1);
    }
}
