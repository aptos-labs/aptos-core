address marketplace {
/// Provides the ability to make token offers to both Tokenv1 and Tokenv2 tokens.
/// A token offer allows an entity to place a bid on a token at any time. The amount
/// offered is extracted from their account and stored at an escrow. A seller can then
/// exchange the token for the escrowed payment. If it is a tokenv2 or the recipient
/// has enabled direct deposit, the token is immediately transferred. If it is tokenv1
/// without direct deposit, it is stored in a container until the recipient extracts it.
module token_offer {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;

    use velor_framework::coin::{Self, Coin};
    use velor_framework::object::{Self, DeleteRef, Object};
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;

    use velor_token_objects::royalty;
    use velor_token_objects::token::{Self as tokenv2, Token as TokenV2};

    use marketplace::events;
    use marketplace::fee_schedule::{Self, FeeSchedule};
    use marketplace::listing::{Self, TokenV1Container};
    use velor_token::token::TokenId;
    use velor_framework::velor_account;

    /// No token offer defined.
    const ENO_TOKEN_OFFER: u64 = 1;
    /// No coin offer defined.
    const ENO_COIN_OFFER: u64 = 2;
    /// This is not the owner of the token.
    const ENOT_TOKEN_OWNER: u64 = 3;
    /// This is not the owner of the token offer.
    const ENOT_OWNER: u64 = 4;
    /// The token offer has expired.
    const EEXPIRED: u64 = 6;

    // Core data structures

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Create a timed offer to buy a token. The token and
    /// assets used to buy are stored in other resources within the object.
    struct TokenOffer has key {
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
        delete_ref: DeleteRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores coins for a token offer.
    struct CoinOffer<phantom CoinType> has key {
        coins: Coin<CoinType>,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores the metadata associated with a tokenv1 token offer.
    struct TokenOfferTokenV1 has copy, drop, key {
        creator_address: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores the metadata associated with a tokenv2 token offer.
    struct TokenOfferTokenV2 has copy, drop, key {
        token: Object<TokenV2>,
    }

    // Initializers

    /// Create a tokenv1 token offer.
    public entry fun init_for_tokenv1_entry<CoinType>(
        purchaser: &signer,
        creator_address: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
    ) {
        init_for_tokenv1<CoinType>(
            purchaser,
            creator_address,
            collection_name,
            token_name,
            property_version,
            fee_schedule,
            item_price,
            expiration_time
        );
    }

    public fun init_for_tokenv1<CoinType>(
        purchaser: &signer,
        creator_address: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
    ): Object<TokenOffer> {
        let offer_signer = init_offer(purchaser, fee_schedule, item_price, expiration_time);
        init_coin_holder<CoinType>(purchaser, &offer_signer, fee_schedule, item_price);
        move_to(&offer_signer, TokenOfferTokenV1 { creator_address, collection_name, token_name, property_version });

        let token_id = tokenv1::create_token_id(
            tokenv1::create_token_data_id(creator_address, collection_name, token_name),
            property_version
        );
        let token_offer_addr = signer::address_of(&offer_signer);
        events::emit_token_offer_placed(
            fee_schedule,
            token_offer_addr,
            signer::address_of(purchaser),
            item_price,
            events::token_metadata_for_tokenv1(token_id),
        );

        object::address_to_object(token_offer_addr)
    }

    /// Create a tokenv2 token offer.
    public entry fun init_for_tokenv2_entry<CoinType>(
        purchaser: &signer,
        token: Object<TokenV2>,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
    ) {
        init_for_tokenv2<CoinType>(
            purchaser,
            token,
            fee_schedule,
            item_price,
            expiration_time
        );
    }

    public fun init_for_tokenv2<CoinType>(
        purchaser: &signer,
        token: Object<TokenV2>,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
    ): Object<TokenOffer> {
        let offer_signer = init_offer(purchaser, fee_schedule, item_price, expiration_time);
        init_coin_holder<CoinType>(purchaser, &offer_signer, fee_schedule, item_price);
        move_to(&offer_signer, TokenOfferTokenV2 { token });

        let token_offer_addr = signer::address_of(&offer_signer);
        events::emit_token_offer_placed(
            fee_schedule,
            token_offer_addr,
            signer::address_of(purchaser),
            item_price,
            events::token_metadata_for_tokenv2(token),
        );

        object::address_to_object(token_offer_addr)
    }

    inline fun init_offer(
        purchaser: &signer,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        expiration_time: u64,
    ): signer {
        let constructor_ref = object::create_object_from_account(purchaser);
        // Once we construct this, both the listing and its contents are soulbound until the conclusion.
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::disable_ungated_transfer(&transfer_ref);

        let offer_signer = object::generate_signer(&constructor_ref);
        let offer = TokenOffer {
            fee_schedule,
            item_price,
            expiration_time,
            delete_ref: object::generate_delete_ref(&constructor_ref),
        };
        move_to(&offer_signer, offer);

        offer_signer
    }

    inline fun init_coin_holder<CoinType>(
        purchaser: &signer,
        offer_signer: &signer,
        fee_schedule: Object<FeeSchedule>,
        total_to_extract: u64,
    ) {
        let fee = fee_schedule::listing_fee(fee_schedule, total_to_extract);
        let fee_address = fee_schedule::fee_address(fee_schedule);
        velor_account::transfer_coins<CoinType>(purchaser, fee_address, fee);

        let coins = coin::withdraw<CoinType>(purchaser, total_to_extract);
        move_to(offer_signer, CoinOffer { coins });
    }

    // Mutators

    ///
    public entry fun cancel<CoinType>(
        purchaser: &signer,
        token_offer: Object<TokenOffer>,
    ) acquires CoinOffer, TokenOffer, TokenOfferTokenV1, TokenOfferTokenV2 {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOffer>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        assert!(
            object::is_owner(token_offer, signer::address_of(purchaser)),
            error::permission_denied(ENOT_OWNER),
        );
        let token_offer_obj = borrow_global_mut<TokenOffer>(token_offer_addr);
        let token_metadata = if (exists<TokenOfferTokenV2>(token_offer_addr)) {
            events::token_metadata_for_tokenv2(
                borrow_global<TokenOfferTokenV2>(token_offer_addr).token,
            )
        } else {
            let offer_info = borrow_global<TokenOfferTokenV1>(token_offer_addr);
            events::token_metadata_for_tokenv1(
                token_v1_token_id(offer_info)
            )
        };

        events::emit_token_offer_canceled(
            token_offer_obj.fee_schedule,
            token_offer_addr,
            signer::address_of(purchaser),
            token_offer_obj.item_price,
            token_metadata,
        );

        cleanup<CoinType>(token_offer);
    }

    /// Sell a tokenv1 to a token offer.
    public entry fun sell_tokenv1_entry<CoinType>(
        seller: &signer,
        token_offer: Object<TokenOffer>,
        token_name: String,
        property_version: u64,
    ) acquires CoinOffer, TokenOffer, TokenOfferTokenV1, TokenOfferTokenV2
    {
        sell_tokenv1<CoinType>(seller, token_offer, token_name, property_version);
    }

    /// Sell a tokenv1 to a token offer.
    public fun sell_tokenv1<CoinType>(
        seller: &signer,
        token_offer: Object<TokenOffer>,
        token_name: String,
        property_version: u64,
    ): Option<Object<TokenV1Container>>
    acquires
    CoinOffer,
    TokenOffer,
    TokenOfferTokenV1,
    TokenOfferTokenV2
    {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOfferTokenV1>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        let token_offer_tokenv1_offer =
            borrow_global_mut<TokenOfferTokenV1>(token_offer_addr);

        // Move the token to its destination

        let token_id = tokenv1::create_token_id_raw(
            token_offer_tokenv1_offer.creator_address,
            token_offer_tokenv1_offer.collection_name,
            token_name,
            property_version,
        );

        let token = tokenv1::withdraw_token(seller, token_id, 1);

        let recipient = object::owner(token_offer);
        let container = if (tokenv1::get_direct_transfer(recipient)) {
            tokenv1::direct_deposit_with_opt_in(recipient, token);
            option::none()
        } else {
            let container = listing::create_tokenv1_container_with_token(seller, token);
            object::transfer(seller, container, recipient);
            option::some(container)
        };

        // Pay fees

        let royalty = tokenv1::get_royalty(token_id);
        settle_payments<CoinType>(
            object::owner(token_offer),
            signer::address_of(seller),
            token_offer_addr,
            tokenv1::get_royalty_payee(&royalty),
            tokenv1::get_royalty_denominator(&royalty),
            tokenv1::get_royalty_numerator(&royalty),
            events::token_metadata_for_tokenv1(token_id),
        );

        container
    }

    /// Sell a tokenv2 to a token offer.
    public entry fun sell_tokenv2<CoinType>(
        seller: &signer,
        token_offer: Object<TokenOffer>,
    ) acquires CoinOffer, TokenOffer, TokenOfferTokenV1, TokenOfferTokenV2 {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOfferTokenV2>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );

        // Check it's the correct token
        let seller_address = signer::address_of(seller);
        let token = borrow_global<TokenOfferTokenV2>(token_offer_addr).token;
        assert!(seller_address == object::owner(token), error::permission_denied(ENOT_TOKEN_OWNER));

        // Move the token to its destination
        let recipient = object::owner(token_offer);
        object::transfer(seller, token, recipient);

        // Pay fees

        let royalty = tokenv2::royalty(token);
        let (royalty_payee, royalty_denominator, royalty_numerator) = if (option::is_some(&royalty)) {
            let royalty = option::destroy_some(royalty);
            let payee_address = royalty::payee_address(&royalty);
            let denominator = royalty::denominator(&royalty);
            let numerator = royalty::numerator(&royalty);
            (payee_address, denominator, numerator)
        } else {
            (signer::address_of(seller), 1, 0)
        };

        settle_payments<CoinType>(
            object::owner(token_offer),
            seller_address,
            token_offer_addr,
            royalty_payee,
            royalty_denominator,
            royalty_numerator,
            events::token_metadata_for_tokenv2(token),
        );
    }

    /// From the coin offer remove appropriate payment for the token and distribute to the seller,
    /// the creator for royalties, and the marketplace for commission. If there are no more slots,
    /// cleanup the offer.
    inline fun settle_payments<CoinType>(
        buyer: address,
        seller: address,
        token_offer_addr: address,
        royalty_payee: address,
        royalty_denominator: u64,
        royalty_numerator: u64,
        token_metadata: events::TokenMetadata,
    ) acquires CoinOffer, TokenOffer, TokenOfferTokenV1, TokenOfferTokenV2 {
        assert!(exists<TokenOffer>(token_offer_addr), error::not_found(ENO_TOKEN_OFFER));
        let token_offer_obj = borrow_global_mut<TokenOffer>(token_offer_addr);
        assert!(
            timestamp::now_seconds() < token_offer_obj.expiration_time,
            error::invalid_state(EEXPIRED),
        );
        let price = token_offer_obj.item_price;

        assert!(
            exists<CoinOffer<CoinType>>(token_offer_addr),
            error::not_found(ENO_COIN_OFFER),
        );
        let coin_offer = borrow_global_mut<CoinOffer<CoinType>>(token_offer_addr);
        let coins = coin::extract(&mut coin_offer.coins, price);

        let royalty_charge = price * royalty_numerator / royalty_denominator;
        let royalties = coin::extract(&mut coins, royalty_charge);
        velor_account::deposit_coins(royalty_payee, royalties);

        let fee_schedule = token_offer_obj.fee_schedule;
        let commission_charge = fee_schedule::commission(fee_schedule, price);
        let commission = coin::extract(&mut coins, commission_charge);
        velor_account::deposit_coins(fee_schedule::fee_address(fee_schedule), commission);

        velor_account::deposit_coins(seller, coins);

        events::emit_token_offer_filled(
            fee_schedule,
            token_offer_addr,
            buyer,
            seller,
            price,
            royalty_charge,
            commission_charge,
            token_metadata,
        );

        cleanup<CoinType>(object::address_to_object(token_offer_addr));
    }

    /// Cleanup the offer by deleting it and returning the remaining funds to the token offer
    /// creator.
    inline fun cleanup<CoinType>(
        token_offer: Object<TokenOffer>,
    ) acquires CoinOffer, TokenOffer, TokenOfferTokenV1, TokenOfferTokenV2 {
        let token_offer_addr = object::object_address(&token_offer);
        let CoinOffer<CoinType> { coins } = move_from(token_offer_addr);
        velor_account::deposit_coins(object::owner(token_offer), coins);

        let TokenOffer {
            fee_schedule: _,
            item_price: _,
            expiration_time: _,
            delete_ref,
        } = move_from(token_offer_addr);
        object::delete(delete_ref);

        if (exists<TokenOfferTokenV2>(token_offer_addr)) {
            move_from<TokenOfferTokenV2>(token_offer_addr);
        } else if (exists<TokenOfferTokenV1>(token_offer_addr)) {
            move_from<TokenOfferTokenV1>(token_offer_addr);
        };
    }

    // View

    #[view]
    public fun exists_at(token_offer: Object<TokenOffer>): bool {
        exists<TokenOffer>(object::object_address(&token_offer))
    }

    #[view]
    public fun expired(token_offer: Object<TokenOffer>): bool acquires TokenOffer {
        borrow_token_offer(token_offer).expiration_time < timestamp::now_seconds()
    }

    #[view]
    public fun expiration_time(
        token_offer: Object<TokenOffer>,
    ): u64 acquires TokenOffer {
        borrow_token_offer(token_offer).expiration_time
    }

    #[view]
    public fun fee_schedule(
        token_offer: Object<TokenOffer>,
    ): Object<FeeSchedule> acquires TokenOffer {
        borrow_token_offer(token_offer).fee_schedule
    }

    #[view]
    public fun price(token_offer: Object<TokenOffer>): u64 acquires TokenOffer {
        borrow_token_offer(token_offer).item_price
    }

    #[view]
    public fun collectionv1(
        token_offer: Object<TokenOffer>,
    ): TokenOfferTokenV1 acquires TokenOfferTokenV1 {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOfferTokenV1>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        *borrow_global(token_offer_addr)
    }

    #[view]
    public fun collectionv2(
        token_offer: Object<TokenOffer>,
    ): TokenOfferTokenV2 acquires TokenOfferTokenV2 {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOffer>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        *borrow_global(token_offer_addr)
    }

    inline fun borrow_token_offer(
        token_offer: Object<TokenOffer>,
    ): &TokenOffer acquires TokenOffer {
        let token_offer_addr = object::object_address(&token_offer);
        assert!(
            exists<TokenOffer>(token_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        borrow_global(token_offer_addr)
    }

    inline fun token_v1_token_id(
        token_offer_tokenv1_offer: &TokenOfferTokenV1,
    ): TokenId {
        tokenv1::create_token_id_raw(
            token_offer_tokenv1_offer.creator_address,
            token_offer_tokenv1_offer.collection_name,
            token_offer_tokenv1_offer.token_name,
            token_offer_tokenv1_offer.property_version,
        )
    }
}

#[test_only]
module token_offer_tests {
    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::coin;
    use velor_framework::object;
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;

    use marketplace::token_offer;
    use marketplace::listing;
    use marketplace::test_utils;
    use std::option;

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_token_v2(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token = test_utils::mint_tokenv2(seller);
        assert!(object::is_owner(token, seller_addr), 0);
        let token_offer = token_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            token,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        assert!(!token_offer::expired(token_offer), 0);
        assert!(token_offer::expiration_time(token_offer) == timestamp::now_seconds() + 200, 0);
        assert!(token_offer::price(token_offer) == 500, 0);

        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9499, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10000, 0);

        token_offer::sell_tokenv2<VelorCoin>(seller, token_offer);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9499, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10495, 0);
        assert!(object::is_owner(token, purchaser_addr), 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_token_v1_direct_deposit(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);
        tokenv1::opt_in_direct_transfer(seller, true);

        let token_id = test_utils::mint_tokenv1(seller);
        assert!(tokenv1::balance_of(seller_addr, token_id) == 1, 0);

        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);

        let token_offer = token_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9499, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10000, 0);

        token_offer::sell_tokenv1<VelorCoin>(seller, token_offer, token_name, property_version);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9499, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10495, 0);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);

        assert!(!token_offer::exists_at(token_offer), 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_token_v1_indirect(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token_id = test_utils::mint_tokenv1(seller);
        assert!(tokenv1::balance_of(seller_addr, token_id) == 1, 0);

        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);

        let token_offer = token_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );

        let token_container = token_offer::sell_tokenv1<VelorCoin>(
            seller,
            token_offer,
            token_name,
            property_version,
        );
        listing::extract_tokenv1(purchaser, option::destroy_some(token_container));
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        assert!(!token_offer::exists_at(token_offer), 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x50003, location = marketplace::token_offer)]
    fun test_token_v2_has_none(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token = test_utils::mint_tokenv2(seller);
        let token_offer = token_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            token,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        token_offer::sell_tokenv2<VelorCoin>(marketplace, token_offer);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10005, location = velor_token::token)]
    fun test_token_v1_has_none(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token_id = test_utils::mint_tokenv1(seller);
        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);

        let token_offer = token_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );

        token_offer::sell_tokenv1<VelorCoin>(
            marketplace,
            token_offer,
            token_name,
            property_version,
        );
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30006, location = marketplace::token_offer)]
    fun test_token_v2_expired(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token = test_utils::mint_tokenv2(seller);
        let token_offer = token_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            token,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        test_utils::increment_timestamp(200);
        token_offer::sell_tokenv2<VelorCoin>(seller, token_offer);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x60001, location = marketplace::token_offer)]
    fun test_token_v2_exhausted(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token = test_utils::mint_tokenv2(seller);
        let token_offer = token_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            token,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        token_offer::sell_tokenv2<VelorCoin>(seller, token_offer);
        token_offer::sell_tokenv2<VelorCoin>(purchaser, token_offer);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x50003, location = marketplace::token_offer)]
    fun test_token_v2_other_token(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let _token = test_utils::mint_tokenv2(seller);
        let token_2 = test_utils::mint_tokenv2_additional(seller);

        let token_offer = token_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            token_2,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        token_offer::sell_tokenv2<VelorCoin>(marketplace, token_offer);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10005, location = velor_token::token)]
    fun test_token_v1_other_token(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token_id_1 = test_utils::mint_tokenv1(seller);
        let (_creator_addr, _collection_name, token_name_1, property_version_1) =
            tokenv1::get_token_id_fields(&token_id_1);

        let token_id_2 = test_utils::mint_tokenv1_additional(seller);
        let (_creator_addr, collection_name, token_name_2, property_version_2) =
            tokenv1::get_token_id_fields(&token_id_2);
        let token_offer = token_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            purchaser_addr,
            collection_name,
            token_name_1,
            property_version_1,
            test_utils::fee_schedule(marketplace),
            500,
            timestamp::now_seconds() + 200,
        );
        token_offer::sell_tokenv1<VelorCoin>(
            marketplace,
            token_offer,
            token_name_2,
            property_version_2,
        );
    }
}
}
