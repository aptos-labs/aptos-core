/// Defines all the events associated with a marketplace. Note: this is attached to a FeeSchedule.
module marketplace::events {
    use std::error;
    use std::option::{Self, Option};
    use std::string::String;

    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::object::{Self, Object};

    use aptos_token::token as tokenv1;
    use aptos_token_objects::collection as collectionv2;
    use aptos_token_objects::token as tokenv2;

    friend marketplace::coin_listing;
    friend marketplace::collection_offer;
    friend marketplace::fee_schedule;
    friend marketplace::listing;
    friend marketplace::token_offer;

    /// Marketplace does not have EventsV1
    const ENO_EVENTS_V1: u64 = 1;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// A holder for all events for a marketplace
    struct EventsV1 has key {
        auction_bid_events: EventHandle<AuctionBidEvent>,
        listing_placed_events: EventHandle<ListingPlacedEvent>,
        listing_canceled_events: EventHandle<ListingCanceledEvent>,
        listing_filled_events: EventHandle<ListingFilledEvent>,

        collection_offer_placed_events: EventHandle<CollectionOfferPlacedEvent>,
        collection_offer_canceled_events: EventHandle<CollectionOfferCanceledEvent>,
        collection_offer_filled_events: EventHandle<CollectionOfferFilledEvent>,

        token_offer_placed_events: EventHandle<TokenOfferPlacedEvent>,
        token_offer_canceled_events: EventHandle<TokenOfferCanceledEvent>,
        token_offer_filled_events: EventHandle<TokenOfferFilledEvent>,
    }

    // Initializers

    public(friend) fun init(fee_schedule_signer: &signer) {
        let events = EventsV1 {
            auction_bid_events: object::new_event_handle(fee_schedule_signer),
            listing_placed_events: object::new_event_handle(fee_schedule_signer),
            listing_canceled_events: object::new_event_handle(fee_schedule_signer),
            listing_filled_events: object::new_event_handle(fee_schedule_signer),
            collection_offer_placed_events: object::new_event_handle(fee_schedule_signer),
            collection_offer_canceled_events: object::new_event_handle(fee_schedule_signer),
            collection_offer_filled_events: object::new_event_handle(fee_schedule_signer),
            token_offer_placed_events: object::new_event_handle(fee_schedule_signer),
            token_offer_canceled_events: object::new_event_handle(fee_schedule_signer),
            token_offer_filled_events: object::new_event_handle(fee_schedule_signer),
        };
        move_to(fee_schedule_signer, events);
    }

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

    /// An event triggered upon each bid.
    struct AuctionBidEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.auction_bid_events, AuctionBidEvent {
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

    struct ListingPlacedEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.listing_placed_events, ListingPlacedEvent {
            type,
            listing,
            seller,
            price,
            token_metadata,
        });
    }

    struct ListingCanceledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.listing_canceled_events, ListingCanceledEvent {
            type,
            listing,
            seller,
            price,
            token_metadata,
        });
    }

    struct ListingFilledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.listing_filled_events, ListingFilledEvent {
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

    struct CollectionOfferPlacedEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.collection_offer_placed_events, CollectionOfferPlacedEvent {
            collection_offer,
            purchaser,
            price,
            token_amount,
            collection_metadata,
        });
    }

    struct CollectionOfferCanceledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.collection_offer_canceled_events, CollectionOfferCanceledEvent {
            collection_offer,
            purchaser,
            price,
            remaining_token_amount,
            collection_metadata,
        });
    }

    struct CollectionOfferFilledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.collection_offer_filled_events, CollectionOfferFilledEvent {
            collection_offer,
            purchaser,
            seller,
            price,
            royalties,
            commission,
            token_metadata,
        });
    }

    // Token offer events
    struct TokenOfferPlacedEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.token_offer_placed_events, TokenOfferPlacedEvent {
            token_offer,
            purchaser,
            price,
            token_metadata,
        });
    }

    struct TokenOfferCanceledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.token_offer_canceled_events, TokenOfferCanceledEvent {
            token_offer,
            purchaser,
            price,
            token_metadata,
        });
    }

    struct TokenOfferFilledEvent has drop, store {
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
    ) acquires EventsV1 {
        let marketplace_events = get_events_v1(marketplace);
        event::emit_event(&mut marketplace_events.token_offer_filled_events, TokenOfferFilledEvent {
            token_offer,
            purchaser,
            seller,
            price,
            royalties,
            commission,
            token_metadata,
        });
    }

    inline fun get_events_v1<T: key>(marketplace: Object<T>): &mut EventsV1 acquires EventsV1 {
        let addr = object::object_address(&marketplace);
        assert!(exists<EventsV1>(addr), error::not_found(ENO_EVENTS_V1));
        borrow_global_mut<EventsV1>(addr)
    }
}
