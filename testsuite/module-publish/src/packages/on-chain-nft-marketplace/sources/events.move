/// Defines all the events associated with a marketplace.
module open_marketplace::events {
    use std::option::{Self, Option};
    use std::string::String;

    use aptos_framework::event;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::fungible_asset::{Self, Metadata as FungibleMetadata};
    use aptos_token_objects::collection;
    use aptos_token_objects::token;

    /// TokenMetadata represents the metadata of a token in events
    struct TokenMetadata has drop, store {
        /// Address of the token creator
        creator_address: address,
        /// Name of the collection
        collection_name: String,
        /// Collection object reference
        collection: Option<Object<collection::Collection>>,
        /// Name of the token
        token_name: String,
        /// Token object reference
        token: Option<Object<token::Token>>,
        /// Property version, if applicable
        property_version: Option<u64>,
    }

    /// Creates TokenMetadata from a tokenv2 token object
    public fun token_metadata(token: Object<token   ::Token>): TokenMetadata {
        TokenMetadata {
            creator_address: token::creator(token),
            collection_name: token::collection_name(token),
            collection: option::some(token::collection_object(token)),
            token_name: token::name(token),
            token: option::some(token),
            property_version: option::none(),
        }
    }

    /// CollectionMetadata represents the metadata of a collection in events
    struct CollectionMetadata has drop, store {
        /// Address of the collection creator
        creator_address: address,
        /// Name of the collection
        collection_name: String,
        /// Collection object reference
        collection: Option<Object<collection::Collection>>,
    }

    /// Creates CollectionMetadata from a tokenv2 collection object
    public fun collection_metadata(
        collection: Object<collection::Collection>,
    ): CollectionMetadata {
        CollectionMetadata {
            creator_address: collection::creator(collection),
            collection_name: collection::name(collection),
            collection: option::some(collection),
        }
    }

    /// PricingInfo represents the pricing information of a FA asset
    struct PricingInfo has drop, store {
        /// Price of the listing
        price: u64,
        /// Address of the fa metadata object used for pricing
        fa_asset_type: address,
        /// Number of decimals for the fa asset
        decimals: u8,
    }

    inline fun create_pricing_info(
        price: u64,
        fee_token_metadata: Object<FungibleMetadata>,
    ): PricingInfo {
        PricingInfo {
            price,
            fa_asset_type: object::object_address(&fee_token_metadata),
            decimals: fungible_asset::decimals(fee_token_metadata),
        }
    }

    // Listing events
    
    // Event emitted when a listing is placed in the marketplace
    #[event]
    struct ListingPlaced has drop, store {
        /// Address of the marketplace integrator
        integrator: address,
        /// Address of the listing object
        listing: address,
        /// Address of the seller
        seller: address,
        /// Pricing information for the listing
        pricing_info: PricingInfo,
        /// Metadata of the token that was listed
        token_metadata: TokenMetadata
    }

    // Event emitted when a listing is canceled
    #[event]
    struct ListingCanceled has drop, store {
        /// Address of the marketplace integrator
        integrator: address,
        /// Address of the listing object
        listing: address,
        /// Address of the seller
        seller: address,
        /// Pricing information for the listing
        pricing_info: PricingInfo,
        /// Metadata of the token that was listed
        token_metadata: TokenMetadata
    }

    // Event emitted when a listing is filled (purchased)
    #[event]
    struct ListingFilled has drop, store {
        /// Address of the marketplace integrator
        integrator: address,
        /// Address of the listing object
        listing: address,
        /// Address of the seller
        seller: address,
        /// Address of the purchaser
        purchaser: address,
        /// Pricing information for the listing
        pricing_info: PricingInfo,
        /// Commission paid to the marketplace
        commission: u64,
        /// Royalties paid to the creator
        royalties: u64,
        /// Metadata of the token that was purchased
        token_metadata: TokenMetadata
    }

    /// Emits a ListingPlaced event
    package fun emit_listing_placed<T: key>(
        integrator: Object<T>, // FeeSchedule for integrator
        listing: address,
        seller: address,
        price: u64,
        fee_token_metadata: Object<FungibleMetadata>,
        token_metadata: TokenMetadata,
    ) {
        let integrator_addr = object::object_address(&integrator);
        let pricing_info = create_pricing_info(price, fee_token_metadata);
        
        event::emit(ListingPlaced {
            integrator: integrator_addr,
            listing,
            seller,
            pricing_info,
            token_metadata,
        });
    }

    /// Emits a ListingCanceled event
    package fun emit_listing_canceled<T: key>(
        integrator: Object<T>, // FeeSchedule for integrator
        listing: address,
        seller: address,
        price: u64,
        fee_token_metadata: Object<FungibleMetadata>,
        token_metadata: TokenMetadata,
    ) {
        let integrator_addr = object::object_address(&integrator);
        let pricing_info = create_pricing_info(price, fee_token_metadata);
        
        event::emit(ListingCanceled {
            integrator: integrator_addr,
            listing,
            seller,
            pricing_info,
            token_metadata,
        });
    }

    /// Emits a ListingFilled event
    package fun emit_listing_filled<T: key>(
        integrator: Object<T>, // FeeSchedule for integrator
        listing: address,
        seller: address,
        purchaser: address,
        price: u64,
        fee_token_metadata: Object<FungibleMetadata>,
        commission: u64,
        royalties: u64,
        token_metadata: TokenMetadata,
    ) {
        let integrator_addr = object::object_address(&integrator);
        let pricing_info = create_pricing_info(price, fee_token_metadata);
        
        event::emit(ListingFilled {
            integrator: integrator_addr,
            listing,
            seller,
            purchaser,
            pricing_info,
            commission,
            royalties,
            token_metadata,
        });
    }
}
