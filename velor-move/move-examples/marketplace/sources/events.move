/// Defines all the events associated with a marketplace. Note: this is attached to a FeeSchedule.
module marketplace::events {
    use std::option::{Self, Option};
    use std::string::String;

    use velor_framework::event;
    use velor_framework::object::{Self, Object};

    use velor_token::token as tokenv1;
    use velor_token_objects::collection as collectionv2;
    use velor_token_objects::token as tokenv2;

    friend marketplace::coin_listing;
    friend marketplace::collection_offer;
    friend marketplace::fee_schedule;
    friend marketplace::listing;
    friend marketplace::token_offer;

    // TokenMetadata and helpers
    struct TokenMetadata has drop, store {
        creator_address: address,
        collection_name: String,
        collection: Option<Object<collectionv2::Collection>>,
        token_name: String,
        token: Option<Object<tokenv2::Token>>,
        property_version: Option<u64>,
    }

    public fun token_metadata_for_tokenv1(token_id: tokenv1::TokenId): TokenMetadata {
        let (creator_address, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);

        TokenMetadata {
            creator_address,
            collection_name,
            collection: option::none(),
            token_name,
            token: option::none(),
            property_version: option::some(property_version),
        }
    }

    public fun token_metadata_for_tokenv2(token: Object<tokenv2::Token>): TokenMetadata {
        TokenMetadata {
            creator_address: tokenv2::creator(token),
            collection_name: tokenv2::collection_name(token),
            collection: option::some(tokenv2::collection_object(token)),
            token_name: tokenv2::name(token),
            token: option::some(token),
            property_version: option::none(),
        }
    }

    // CollectionMetadata and helpers

    struct CollectionMetadata has drop, store {
        creator_address: address,
        collection_name: String,
        collection: Option<Object<collectionv2::Collection>>,
    }

    public fun collection_metadata_for_tokenv1(
        creator_address: address,
        collection_name: String,
    ): CollectionMetadata {
        CollectionMetadata {
            creator_address,
            collection_name,
            collection: option::none(),
        }
    }

    public fun collection_metadata_for_tokenv2(
        collection: Object<collectionv2::Collection>,
    ): CollectionMetadata {
        CollectionMetadata {
            creator_address: collectionv2::creator(collection),
            collection_name: collectionv2::name(collection),
            collection: option::some(collection),
        }
    }

    // Listing events

    #[event]
    /// An event triggered upon each bid.
    struct AuctionBid has drop, store {
        marketplace: address,
        listing: address,
        new_bidder: address,
        new_bid: u64,
        new_end_time: u64,
        previous_bidder: Option<address>,
        previous_bid: Option<u64>,
        previous_end_time: u64,
        token_metadata: TokenMetadata,
    }

    public(friend) fun emit_bid_event<T: key>(
        marketplace: Object<T>,
        listing: address,
        new_bidder: address,
        new_bid: u64,
        new_end_time: u64,
        previous_bidder: Option<address>,
        previous_bid: Option<u64>,
        previous_end_time: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(AuctionBid {
            marketplace: object::object_address(&marketplace),
            listing,
            new_bidder,
            new_bid,
            new_end_time,
            previous_bidder,
            previous_bid,
            previous_end_time,
            token_metadata,
        });
    }

    #[event]
    struct ListingPlaced has drop, store {
        marketplace: address,
        type: String,
        listing: address,
        seller: address,
        price: u64,
        token_metadata: TokenMetadata
    }

    public(friend) fun emit_listing_placed<T: key>(
        marketplace: Object<T>,
        type: String,
        listing: address,
        seller: address,
        price: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(ListingPlaced {
            marketplace: object::object_address(&marketplace),
            type,
            listing,
            seller,
            price,
            token_metadata,
        });
    }

    #[event]
    struct ListingCanceled has drop, store {
        marketplace: address,
        type: String,
        listing: address,
        seller: address,
        price: u64,
        token_metadata: TokenMetadata
    }

    public(friend) fun emit_listing_canceled<T: key>(
        marketplace: Object<T>,
        type: String,
        listing: address,
        seller: address,
        price: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(ListingCanceled {
            marketplace: object::object_address(&marketplace),
            type,
            listing,
            seller,
            price,
            token_metadata,
        });
    }

    #[event]
    struct ListingFilled has drop, store {
        marketplace: address,
        type: String,
        listing: address,
        seller: address,
        purchaser: address,
        price: u64,
        commission: u64,
        royalties: u64,
        token_metadata: TokenMetadata
    }

    public(friend) fun emit_listing_filled<T: key>(
        marketplace: Object<T>,
        type: String,
        listing: address,
        seller: address,
        purchaser: address,
        price: u64,
        commission: u64,
        royalties: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(ListingFilled {
            marketplace: object::object_address(&marketplace),
            type,
            listing,
            seller,
            purchaser,
            price,
            commission,
            royalties,
            token_metadata,
        });
    }

    // Collection offer events

    #[event]
    struct CollectionOfferPlaced has drop, store {
        marketplace: address,
        collection_offer: address,
        purchaser: address,
        price: u64,
        token_amount: u64,
        collection_metadata: CollectionMetadata,
    }

    public(friend) fun emit_collection_offer_placed<T: key>(
        marketplace: Object<T>,
        collection_offer: address,
        purchaser: address,
        price: u64,
        token_amount: u64,
        collection_metadata: CollectionMetadata,
    ) {
        event::emit(CollectionOfferPlaced {
            marketplace: object::object_address(&marketplace),
            collection_offer,
            purchaser,
            price,
            token_amount,
            collection_metadata,
        });
    }

    #[event]
    struct CollectionOfferCanceled has drop, store {
        marketplace: address,
        collection_offer: address,
        purchaser: address,
        price: u64,
        remaining_token_amount: u64,
        collection_metadata: CollectionMetadata,
    }

    public(friend) fun emit_collection_offer_canceled<T: key>(
        marketplace: Object<T>,
        collection_offer: address,
        purchaser: address,
        price: u64,
        remaining_token_amount: u64,
        collection_metadata: CollectionMetadata,
    ) {
        event::emit(CollectionOfferCanceled {
            marketplace: object::object_address(&marketplace),
            collection_offer,
            purchaser,
            price,
            remaining_token_amount,
            collection_metadata,
        });
    }

    #[event]
    struct CollectionOfferFilled has drop, store {
        marketplace: address,
        collection_offer: address,
        purchaser: address,
        seller: address,
        price: u64,
        royalties: u64,
        commission: u64,
        token_metadata: TokenMetadata,
    }

    public(friend) fun emit_collection_offer_filled<T: key>(
        marketplace: Object<T>,
        collection_offer: address,
        purchaser: address,
        seller: address,
        price: u64,
        royalties: u64,
        commission: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(CollectionOfferFilled {
            marketplace: object::object_address(&marketplace),
            collection_offer,
            purchaser,
            seller,
            price,
            royalties,
            commission,
            token_metadata,
        });
    }

    #[event]
    // Token offer events
    struct TokenOfferPlaced has drop, store {
        marketplace: address,
        token_offer: address,
        purchaser: address,
        price: u64,
        token_metadata: TokenMetadata,
    }

    public(friend) fun emit_token_offer_placed<T: key>(
        marketplace: Object<T>,
        token_offer: address,
        purchaser: address,
        price: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(TokenOfferPlaced {
            marketplace: object::object_address(&marketplace),
            token_offer,
            purchaser,
            price,
            token_metadata,
        });
    }

    #[event]
    struct TokenOfferCanceled has drop, store {
        marketplace: address,
        token_offer: address,
        purchaser: address,
        price: u64,
        token_metadata: TokenMetadata,
    }

    public(friend) fun emit_token_offer_canceled<T: key>(
        marketplace: Object<T>,
        token_offer: address,
        purchaser: address,
        price: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(TokenOfferCanceled {
            marketplace: object::object_address(&marketplace),
            token_offer,
            purchaser,
            price,
            token_metadata,
        });
    }

    #[event]
    struct TokenOfferFilled has drop, store {
        marketplace: address,
        token_offer: address,
        purchaser: address,
        seller: address,
        price: u64,
        royalties: u64,
        commission: u64,
        token_metadata: TokenMetadata,
    }

    public(friend) fun emit_token_offer_filled<T: key>(
        marketplace: Object<T>,
        token_offer: address,
        purchaser: address,
        seller: address,
        price: u64,
        royalties: u64,
        commission: u64,
        token_metadata: TokenMetadata,
    ) {
        event::emit(TokenOfferFilled {
            marketplace: object::object_address(&marketplace),
            token_offer,
            purchaser,
            seller,
            price,
            royalties,
            commission,
            token_metadata,
        });
    }
}
