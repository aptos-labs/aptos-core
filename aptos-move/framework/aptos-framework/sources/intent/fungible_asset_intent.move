module aptos_framework::fungible_asset_intent {
    use std::error;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata, FungibleStore};
    use aptos_framework::intent::{Self, TradeSession, TradeIntent};
    use aptos_framework::object::{Self, DeleteRef, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;

    /// The token offered is not the desired fungible asset.
    const ENOT_DESIRED_TOKEN: u64 = 0;

    /// The token offered does not meet amount requirement.
    const EAMOUNT_NOT_MEET: u64 = 1;

    struct FungibleStoreManager has store {
        extend_ref: ExtendRef,
        delete_ref: DeleteRef,
    }

    struct FungibleAssetLimitOrder has store, drop {
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        issuer: address,
    }

    struct FungibleAssetRecipientWitness has drop {}

    public fun create_fa_to_fa_intent(
        source_fungible_asset: FungibleAsset,
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        expiry_time: u64,
        issuer: address,
    ): Object<TradeIntent<FungibleStoreManager, FungibleAssetLimitOrder>> {
        let coin_store_ref = object::create_self_owned_object();
        let extend_ref = object::generate_extend_ref(&coin_store_ref);
        let delete_ref = object::generate_delete_ref(&coin_store_ref);
        fungible_asset::create_store(&coin_store_ref, fungible_asset::metadata_from_asset(&source_fungible_asset));
        fungible_asset::deposit(
            object::object_from_constructor_ref<FungibleStore>(&coin_store_ref),
            source_fungible_asset
        );
        intent::create_intent<FungibleStoreManager, FungibleAssetLimitOrder, FungibleAssetRecipientWitness>(
            FungibleStoreManager { extend_ref, delete_ref},
            FungibleAssetLimitOrder { desired_metadata, desired_amount, issuer },
            expiry_time,
            issuer,
            FungibleAssetRecipientWitness {},
        )
    }

    public fun start_fa_offering_session<Args: store + drop>(
        intent: Object<TradeIntent<FungibleStoreManager, Args>>
    ): (FungibleAsset, TradeSession<Args>) {
        let (store_manager, session) = intent::start_intent_session(intent);
        let FungibleStoreManager { extend_ref, delete_ref } = store_manager;
        let store_signer = object::generate_signer_for_extending(&extend_ref);
        let fa_store = object::object_from_delete_ref<FungibleStore>(&delete_ref);
        let fa = fungible_asset::withdraw(&store_signer, fa_store, fungible_asset::balance(fa_store));
        fungible_asset::remove_store(&delete_ref);
        object::delete(delete_ref);
        (fa, session)
    }

    public fun finish_fa_receiving_session(
        session: TradeSession<FungibleAssetLimitOrder>,
        received_fa: FungibleAsset,
    ) {
        let argument = intent::get_argument(&session);
        assert!(
            fungible_asset::metadata_from_asset(&received_fa) == argument.desired_metadata,
            error::invalid_argument(ENOT_DESIRED_TOKEN)
        );
        assert!(
            fungible_asset::amount(&received_fa) >= argument.desired_amount,
            error::invalid_argument(EAMOUNT_NOT_MEET),
        );

        primary_fungible_store::deposit(argument.issuer, received_fa);
        intent::finish_intent_session(session, FungibleAssetRecipientWitness {})
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

        let (fa1, session) = start_fa_offering_session(intent);

        assert!(fungible_asset::metadata_from_asset(&fa1) == test_token_1, 1);
        assert!(fungible_asset::amount(&fa1) == 10, 1);

        // Mint FA2 for the sake of testing. In the real life we expect a DeFi app to perform the trade.
        let fa2 = fungible_asset::mint(&mint_ref_2, 5);
        fungible_asset::burn(&burn_ref_1, fa1);

        // Trade FA1 for 5 FA2
        finish_fa_receiving_session(session, fa2);

        assert!(primary_fungible_store::balance(signer::address_of(aaron), test_token_2) == 5, 1);
    }
}
