address marketplace {
/// Provides the ability to make collection offers to both Tokenv1 and Tokenv2 collections.
/// A collection offer allows an entity to buy up to N assets within a collection at their
/// specified amount. The amount offered is extracted from their account and stored at an
/// escrow. A seller can then exchange the token for the escrowed payment. If it is a
/// a tokenv2 or the recipient has enabled direct deposit, the token is immediately
/// transferred. If it is tokenv1 without direct deposit, it is stored in a container
/// until the recipient extracts it.
module collection_offer {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;
    use velor_std::math64;

    use velor_framework::coin::{Self, Coin};
    use velor_framework::object::{Self, DeleteRef, Object};
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;

    use velor_token_objects::collection::Collection;
    use velor_token_objects::royalty;
    use velor_token_objects::token::{Self as tokenv2, Token as TokenV2};

    use marketplace::events;
    use marketplace::fee_schedule::{Self, FeeSchedule};
    use marketplace::listing::{Self, TokenV1Container};
    use velor_framework::velor_account;

    /// No collection offer defined.
    const ENO_COLLECTION_OFFER: u64 = 1;
    /// No coin offer defined.
    const ENO_COIN_OFFER: u64 = 2;
    /// No token offer defined.
    const ENO_TOKEN_OFFER: u64 = 3;
    /// This is not the owner of the collection offer.
    const ENOT_OWNER: u64 = 4;
    /// The offered token is not within the expected collection.
    const EINCORRECT_COLLECTION: u64 = 5;
    /// The collection offer has expired.
    const EEXPIRED: u64 = 6;

    // Core data structures

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Create a timed offer to buy tokens from a collection. The collection and
    /// assets used to buy are stored in other resources within the object.
    struct CollectionOffer has key {
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        remaining: u64,
        expiration_time: u64,
        delete_ref: DeleteRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores coins for a collection offer.
    struct CoinOffer<phantom CoinType> has key {
        coins: Coin<CoinType>,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores the metadata associated with a tokenv1 collection offer.
    struct CollectionOfferTokenV1 has copy, drop, key {
        creator_address: address,
        collection_name: String,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Stores the metadata associated with a tokenv2 collection offer.
    struct CollectionOfferTokenV2 has copy, drop, key {
        collection: Object<Collection>,
    }

    // Initializers

    /// Create a tokenv1 collection offer.
    public entry fun init_for_tokenv1_entry<CoinType>(
        purchaser: &signer,
        creator_address: address,
        collection_name: String,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        amount: u64,
        expiration_time: u64,
    ) {
        init_for_tokenv1<CoinType>(
            purchaser,
            creator_address,
            collection_name,
            fee_schedule,
            item_price,
            amount,
            expiration_time
        );
    }

    public fun init_for_tokenv1<CoinType>(
        purchaser: &signer,
        creator_address: address,
        collection_name: String,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        amount: u64,
        expiration_time: u64,
    ): Object<CollectionOffer> {
        let offer_signer = init_offer(purchaser, fee_schedule, item_price, amount, expiration_time);
        init_coin_holder<CoinType>(purchaser, &offer_signer, fee_schedule, item_price * amount);
        move_to(&offer_signer, CollectionOfferTokenV1 { creator_address, collection_name });

        let collection_offer_addr = signer::address_of(&offer_signer);
        events::emit_collection_offer_placed(
            fee_schedule,
            collection_offer_addr,
            signer::address_of(purchaser),
            item_price,
            amount,
            events::collection_metadata_for_tokenv1(creator_address, collection_name),
        );

        object::address_to_object(collection_offer_addr)
    }

    /// Create a tokenv2 collection offer.
    public entry fun init_for_tokenv2_entry<CoinType>(
        purchaser: &signer,
        collection: Object<Collection>,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        amount: u64,
        expiration_time: u64,
    ) {
        init_for_tokenv2<CoinType>(
            purchaser,
            collection,
            fee_schedule,
            item_price,
            amount,
            expiration_time
        );
    }

    public fun init_for_tokenv2<CoinType>(
        purchaser: &signer,
        collection: Object<Collection>,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        amount: u64,
        expiration_time: u64,
    ): Object<CollectionOffer> {
        let offer_signer = init_offer(purchaser, fee_schedule, item_price, amount, expiration_time);
        init_coin_holder<CoinType>(purchaser, &offer_signer, fee_schedule, item_price * amount);
        move_to(&offer_signer, CollectionOfferTokenV2 { collection });

        let collection_offer_addr = signer::address_of(&offer_signer);
        events::emit_collection_offer_placed(
            fee_schedule,
            collection_offer_addr,
            signer::address_of(purchaser),
            item_price,
            amount,
            events::collection_metadata_for_tokenv2(collection),
        );

        object::address_to_object(collection_offer_addr)
    }

    inline fun init_offer(
        purchaser: &signer,
        fee_schedule: Object<FeeSchedule>,
        item_price: u64,
        amount: u64,
        expiration_time: u64,
    ): signer {
        let constructor_ref = object::create_object_from_account(purchaser);
        // Once we construct this, both the listing and its contents are soulbound until the conclusion.
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::disable_ungated_transfer(&transfer_ref);

        let offer_signer = object::generate_signer(&constructor_ref);
        let offer = CollectionOffer {
            fee_schedule,
            item_price,
            remaining: amount,
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
        collection_offer: Object<CollectionOffer>,
    ) acquires CoinOffer, CollectionOffer, CollectionOfferTokenV1, CollectionOfferTokenV2 {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOffer>(collection_offer_addr),
            error::not_found(ENO_COLLECTION_OFFER),
        );
        assert!(
            object::is_owner(collection_offer, signer::address_of(purchaser)),
            error::permission_denied(ENOT_OWNER),
        );
        let collection_offer_obj = borrow_global_mut<CollectionOffer>(collection_offer_addr);
        let collection_metadata = if (exists<CollectionOfferTokenV2>(collection_offer_addr)) {
            events::collection_metadata_for_tokenv2(
                borrow_global<CollectionOfferTokenV2>(collection_offer_addr).collection,
            )
        } else {
            let offer_info = borrow_global<CollectionOfferTokenV1>(collection_offer_addr);
            events::collection_metadata_for_tokenv1(
                offer_info.creator_address,
                offer_info.collection_name,
            )
        };

        events::emit_collection_offer_canceled(
            collection_offer_obj.fee_schedule,
            collection_offer_addr,
            signer::address_of(purchaser),
            collection_offer_obj.item_price,
            collection_offer_obj.remaining,
            collection_metadata,
        );

        cleanup<CoinType>(collection_offer);
    }

    /// Sell a tokenv1 to a collection offer.
    public entry fun sell_tokenv1_entry<CoinType>(
        seller: &signer,
        collection_offer: Object<CollectionOffer>,
        token_name: String,
        property_version: u64,
    ) acquires CoinOffer, CollectionOffer, CollectionOfferTokenV1, CollectionOfferTokenV2
    {
        sell_tokenv1<CoinType>(seller, collection_offer, token_name, property_version);
    }

    /// Sell a tokenv1 to a collection offer.
    public fun sell_tokenv1<CoinType>(
        seller: &signer,
        collection_offer: Object<CollectionOffer>,
        token_name: String,
        property_version: u64,
    ): Option<Object<TokenV1Container>>
    acquires
    CoinOffer,
    CollectionOffer,
    CollectionOfferTokenV1,
    CollectionOfferTokenV2
    {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOfferTokenV1>(collection_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        let collection_offer_tokenv1_offer =
            borrow_global_mut<CollectionOfferTokenV1>(collection_offer_addr);

        // Move the token to its destination

        let token_id = tokenv1::create_token_id_raw(
            collection_offer_tokenv1_offer.creator_address,
            collection_offer_tokenv1_offer.collection_name,
            token_name,
            property_version,
        );

        let token = tokenv1::withdraw_token(seller, token_id, 1);

        let recipient = object::owner(collection_offer);
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
            object::owner(collection_offer),
            signer::address_of(seller),
            collection_offer_addr,
            tokenv1::get_royalty_payee(&royalty),
            tokenv1::get_royalty_denominator(&royalty),
            tokenv1::get_royalty_numerator(&royalty),
            events::token_metadata_for_tokenv1(token_id),
        );

        container
    }

    /// Sell a tokenv2 to a collection offer.
    public entry fun sell_tokenv2<CoinType>(
        seller: &signer,
        collection_offer: Object<CollectionOffer>,
        token: Object<TokenV2>,
    ) acquires CoinOffer, CollectionOffer, CollectionOfferTokenV1, CollectionOfferTokenV2 {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOfferTokenV2>(collection_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        let collection_offer_token_v2 =
            borrow_global_mut<CollectionOfferTokenV2>(collection_offer_addr);

        // Move the token to its destination

        assert!(
            tokenv2::collection_object(token) == collection_offer_token_v2.collection,
            error::invalid_argument(EINCORRECT_COLLECTION),
        );
        let recipient = object::owner(collection_offer);
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
            object::owner(collection_offer),
            signer::address_of(seller),
            collection_offer_addr,
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
        collection_offer_addr: address,
        royalty_payee: address,
        royalty_denominator: u64,
        royalty_numerator: u64,
        token_metadata: events::TokenMetadata,
    ) acquires CoinOffer, CollectionOffer, CollectionOfferTokenV1, CollectionOfferTokenV2 {
        assert!(exists<CollectionOffer>(collection_offer_addr), error::not_found(ENO_COLLECTION_OFFER));
        let collection_offer_obj = borrow_global_mut<CollectionOffer>(collection_offer_addr);
        assert!(
            timestamp::now_seconds() < collection_offer_obj.expiration_time,
            error::invalid_state(EEXPIRED),
        );
        let price = collection_offer_obj.item_price;

        assert!(
            exists<CoinOffer<CoinType>>(collection_offer_addr),
            error::not_found(ENO_COIN_OFFER),
        );
        let coin_offer = borrow_global_mut<CoinOffer<CoinType>>(collection_offer_addr);
        let coins = coin::extract(&mut coin_offer.coins, price);

        let royalty_charge = listing::bounded_percentage(price, royalty_numerator, royalty_denominator);

        let royalties = coin::extract(&mut coins, royalty_charge);
        velor_account::deposit_coins(royalty_payee, royalties);

        // Commission can only be of whatever is left
        let fee_schedule = collection_offer_obj.fee_schedule;
        let commission_charge = fee_schedule::commission(fee_schedule, price);
        let actual_commission_charge = math64::min(commission_charge, coin::value(&coins));
        let commission = coin::extract(&mut coins, actual_commission_charge);
        velor_account::deposit_coins(fee_schedule::fee_address(fee_schedule), commission);

        // Seller gets what is left
        velor_account::deposit_coins(seller, coins);

        events::emit_collection_offer_filled(
            fee_schedule,
            collection_offer_addr,
            buyer,
            seller,
            price,
            royalty_charge,
            commission_charge,
            token_metadata,
        );

        collection_offer_obj.remaining = collection_offer_obj.remaining - 1;
        if (collection_offer_obj.remaining == 0) {
            cleanup<CoinType>(object::address_to_object(collection_offer_addr));
        };
    }

    /// Cleanup the offer by deleting it and returning the remaining funds to the collection offer
    /// creator.
    inline fun cleanup<CoinType>(
        collection_offer: Object<CollectionOffer>,
    ) acquires CoinOffer, CollectionOffer, CollectionOfferTokenV1, CollectionOfferTokenV2 {
        let collection_offer_addr = object::object_address(&collection_offer);
        let CoinOffer<CoinType> { coins } = move_from(collection_offer_addr);
        velor_account::deposit_coins(object::owner(collection_offer), coins);

        let CollectionOffer {
            fee_schedule: _,
            item_price: _,
            remaining: _,
            expiration_time: _,
            delete_ref,
        } = move_from(collection_offer_addr);
        object::delete(delete_ref);

        if (exists<CollectionOfferTokenV2>(collection_offer_addr)) {
            move_from<CollectionOfferTokenV2>(collection_offer_addr);
        } else if (exists<CollectionOfferTokenV1>(collection_offer_addr)) {
            move_from<CollectionOfferTokenV1>(collection_offer_addr);
        };
    }

    // View

    #[view]
    public fun exists_at(collection_offer: Object<CollectionOffer>): bool {
        exists<CollectionOffer>(object::object_address(&collection_offer))
    }

    #[view]
    public fun expired(collection_offer: Object<CollectionOffer>): bool acquires CollectionOffer {
        borrow_collection_offer(collection_offer).expiration_time < timestamp::now_seconds()
    }

    #[view]
    public fun expiration_time(
        collection_offer: Object<CollectionOffer>,
    ): u64 acquires CollectionOffer {
        borrow_collection_offer(collection_offer).expiration_time
    }

    #[view]
    public fun fee_schedule(
        collection_offer: Object<CollectionOffer>,
    ): Object<FeeSchedule> acquires CollectionOffer {
        borrow_collection_offer(collection_offer).fee_schedule
    }

    #[view]
    public fun price(collection_offer: Object<CollectionOffer>): u64 acquires CollectionOffer {
        borrow_collection_offer(collection_offer).item_price
    }

    #[view]
    public fun remaining(collection_offer: Object<CollectionOffer>): u64 acquires CollectionOffer {
        borrow_collection_offer(collection_offer).remaining
    }

    #[view]
    public fun collectionv1(
        collection_offer: Object<CollectionOffer>,
    ): CollectionOfferTokenV1 acquires CollectionOfferTokenV1 {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOfferTokenV1>(collection_offer_addr),
            error::not_found(ENO_TOKEN_OFFER),
        );
        *borrow_global(collection_offer_addr)
    }

    #[view]
    public fun collectionv2(
        collection_offer: Object<CollectionOffer>,
    ): CollectionOfferTokenV2 acquires CollectionOfferTokenV2 {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOffer>(collection_offer_addr),
            error::not_found(ENO_COLLECTION_OFFER),
        );
        *borrow_global(collection_offer_addr)
    }

    inline fun borrow_collection_offer(
        collection_offer: Object<CollectionOffer>,
    ): &CollectionOffer acquires CollectionOffer {
        let collection_offer_addr = object::object_address(&collection_offer);
        assert!(
            exists<CollectionOffer>(collection_offer_addr),
            error::not_found(ENO_COLLECTION_OFFER),
        );
        borrow_global(collection_offer_addr)
    }
}

#[test_only]
module collection_offer_tests {
    use std::string;
    use std::option;

    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::coin;
    use velor_framework::object;
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;

    use velor_token_objects::collection as collectionv2;

    use marketplace::collection_offer;
    use marketplace::listing;
    use marketplace::test_utils;

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_token_v2(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let (collection, token) = test_utils::mint_tokenv2_with_collection(seller);
        assert!(object::is_owner(token, seller_addr), 0);
        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            collection,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        assert!(!collection_offer::expired(collection_offer), 0);
        assert!(collection_offer::expiration_time(collection_offer) == timestamp::now_seconds() + 200, 0);
        assert!(collection_offer::price(collection_offer) == 500, 0);

        assert!(collection_offer::remaining(collection_offer) == 2, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 8999, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10000, 0);

        collection_offer::sell_tokenv2<VelorCoin>(seller, collection_offer, token);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 8999, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10495, 0);
        assert!(object::is_owner(token, purchaser_addr), 0);
        assert!(collection_offer::remaining(collection_offer) == 1, 0);

        collection_offer::sell_tokenv2<VelorCoin>(purchaser, collection_offer, token);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 11, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9489, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10500, 0);
        assert!(object::is_owner(token, purchaser_addr), 0);
        assert!(!collection_offer::exists_at(collection_offer), 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_token_v2_high_royalty(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let (collection, token) = test_utils::mint_tokenv2_with_collection_royalty(seller, 1, 1);
        assert!(object::is_owner(token, seller_addr), 0);
        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            collection,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        assert!(!collection_offer::expired(collection_offer), 0);
        assert!(collection_offer::expiration_time(collection_offer) == timestamp::now_seconds() + 200, 0);
        assert!(collection_offer::price(collection_offer) == 500, 0);

        assert!(collection_offer::remaining(collection_offer) == 2, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 8999, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10000, 0);

        collection_offer::sell_tokenv2<VelorCoin>(seller, collection_offer, token);
        assert!(object::is_owner(token, purchaser_addr), 0);
        assert!(collection_offer::remaining(collection_offer) == 1, 0);

        collection_offer::sell_tokenv2<VelorCoin>(purchaser, collection_offer, token);
        assert!(object::is_owner(token, purchaser_addr), 0);
        assert!(!collection_offer::exists_at(collection_offer), 0);
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

        let collection_offer = collection_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );

        assert!(collection_offer::remaining(collection_offer) == 2, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 8999, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10000, 0);

        collection_offer::sell_tokenv1<VelorCoin>(seller, collection_offer, token_name, property_version);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 8999, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10495, 0);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        assert!(collection_offer::remaining(collection_offer) == 1, 0);

        collection_offer::sell_tokenv1<VelorCoin>(purchaser, collection_offer, token_name, property_version);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 11, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9489, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10500, 0);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        assert!(!collection_offer::exists_at(collection_offer), 0);
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

        let collection_offer = collection_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            test_utils::fee_schedule(marketplace),
            500,
            1,
            timestamp::now_seconds() + 200,
        );

        let token_container = collection_offer::sell_tokenv1<VelorCoin>(
            seller,
            collection_offer,
            token_name,
            property_version,
        );
        listing::extract_tokenv1(purchaser, option::destroy_some(token_container));
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        assert!(!collection_offer::exists_at(collection_offer), 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x50004, location = velor_framework::object)]
    fun test_token_v2_has_none(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let (collection, token) = test_utils::mint_tokenv2_with_collection(seller);
        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            collection,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        collection_offer::sell_tokenv2<VelorCoin>(marketplace, collection_offer, token);
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

        let collection_offer = collection_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            creator_addr,
            collection_name,
            test_utils::fee_schedule(marketplace),
            500,
            1,
            timestamp::now_seconds() + 200,
        );

        collection_offer::sell_tokenv1<VelorCoin>(
            marketplace,
            collection_offer,
            token_name,
            property_version,
        );
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30006, location = marketplace::collection_offer)]
    fun test_token_v2_expired(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let (collection, token) = test_utils::mint_tokenv2_with_collection(seller);
        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            collection,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        test_utils::increment_timestamp(200);
        collection_offer::sell_tokenv2<VelorCoin>(seller, collection_offer, token);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x60003, location = marketplace::collection_offer)]
    fun test_token_v2_exhausted(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let (collection, token) = test_utils::mint_tokenv2_with_collection(seller);
        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            collection,
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        collection_offer::sell_tokenv2<VelorCoin>(seller, collection_offer, token);
        collection_offer::sell_tokenv2<VelorCoin>(purchaser, collection_offer, token);
        collection_offer::sell_tokenv2<VelorCoin>(purchaser, collection_offer, token);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10005, location = marketplace::collection_offer)]
    fun test_token_v2_other_collection(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);
        let token = test_utils::mint_tokenv2(seller);

        let other_collection = collectionv2::create_unlimited_collection(
            purchaser,
            string::utf8(b"..."),
            string::utf8(b"..."),
            option::none(),
            string::utf8(b"..."),
        );

        let collection_offer = collection_offer::init_for_tokenv2<VelorCoin>(
            purchaser,
            object::object_from_constructor_ref(&other_collection),
            test_utils::fee_schedule(marketplace),
            500,
            2,
            timestamp::now_seconds() + 200,
        );
        collection_offer::sell_tokenv2<VelorCoin>(marketplace, collection_offer, token);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10005, location = velor_token::token)]
    fun test_token_v1_other_collection(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        tokenv1::create_collection(
            purchaser,
            string::utf8(b"..."),
            string::utf8(b"..."),
            string::utf8(b"..."),
            1,
            vector[true, true, true],
        );

        let collection_offer = collection_offer::init_for_tokenv1<VelorCoin>(
            purchaser,
            purchaser_addr,
            string::utf8(b"..."),
            test_utils::fee_schedule(marketplace),
            500,
            1,
            timestamp::now_seconds() + 200,
        );

        let token_id = test_utils::mint_tokenv1(seller);
        let (_creator_addr, _collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);
        collection_offer::sell_tokenv1<VelorCoin>(
            marketplace,
            collection_offer,
            token_name,
            property_version,
        );
    }
}
}
