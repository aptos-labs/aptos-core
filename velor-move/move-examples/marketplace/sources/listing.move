/// Defines a single listing or an item for sale or auction. This is an escrow service that
/// enables two parties to exchange one asset for another.
/// Each listing has the following properties:
/// * FeeSchedule specifying payment flows
/// * Owner or the person that can end the sale or auction
/// * Starting time
/// * Logic for cleanup
module marketplace::listing {
    use std::error;
    use std::option;
    use std::signer;
    use std::string::String;

    use velor_std::math64;

    use velor_framework::object::{Self, ConstructorRef, DeleteRef, ExtendRef, Object, ObjectCore, TransferRef};
    use velor_framework::timestamp;

    use velor_token::token::{Self as tokenv1, Token as TokenV1};
    use velor_token_objects::token as tokenv2;
    use velor_token_objects::royalty;

    use marketplace::events;
    use marketplace::fee_schedule::FeeSchedule;

    friend marketplace::coin_listing;

    /// There exists no listing.
    const ENO_LISTING: u64 = 1;
    /// The listing is not yet live.
    const ELISTING_NOT_STARTED: u64 = 2;
    /// The entity is not the creator.
    const ENOT_CREATOR: u64 = 3;
    /// The entity is not the owner of the wrapped token.
    const ENOT_OWNER: u64 = 4;

    // Core data structures

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Corner-stone for all listings, represents the core utility layer including object
    /// cleanup.
    struct Listing has key {
        /// The item owned by this listing, transferred to the new owner at the end.
        object: Object<ObjectCore>,
        /// The seller of the object
        seller: address,
        /// The fees associated with claiming this listing.
        fee_schedule: Object<FeeSchedule>,
        /// The Unix timestamp in seconds at which point bidding and purchasing can occur
        start_time: u64,
        /// Used to clean-up at the end.
        delete_ref: DeleteRef,
        /// Used to create a signer to transfer the listed item, ideally the TransferRef would
        /// support this.
        extend_ref: ExtendRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Contains a tokenv1 as an object
    struct TokenV1Container has key {
        /// The stored token.
        token: TokenV1,
        /// Used to cleanup the object at the end
        delete_ref: DeleteRef,
        /// Used to transfer the tokenv1 at the conclusion of a purchase.
        transfer_ref: TransferRef,
    }

    // Init functions

    public(friend) fun init(
        creator: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
    ): (signer, ConstructorRef) {
        let constructor_ref = object::create_object_from_account(creator);
        // Once we construct this, both the listing and its contents are soulbound until the conclusion.
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::disable_ungated_transfer(&transfer_ref);
        let listing_signer = object::generate_signer(&constructor_ref);

        let listing = Listing {
            object,
            seller: signer::address_of(creator),
            fee_schedule,
            start_time,
            delete_ref: object::generate_delete_ref(&constructor_ref),
            extend_ref: object::generate_extend_ref(&constructor_ref),
        };
        move_to(&listing_signer, listing);

        let listing_addr = object::address_from_constructor_ref(&constructor_ref);
        object::transfer(creator, object, listing_addr);

        (listing_signer, constructor_ref)
    }

    public(friend) fun create_tokenv1_container(
        seller: &signer,
        token_creator: address,
        token_collection: String,
        token_name: String,
        token_property_version: u64,
    ): Object<TokenV1Container> {
        let token_id = tokenv1::create_token_id_raw(
            token_creator,
            token_collection,
            token_name,
            token_property_version,
        );
        let token = tokenv1::withdraw_token(seller, token_id, 1);
        create_tokenv1_container_with_token(seller, token)
    }

    public fun create_tokenv1_container_with_token(
        seller: &signer,
        token: TokenV1,
    ): Object<TokenV1Container> {
        let constructor_ref = object::create_object_from_account(seller);
        let container_signer = object::generate_signer(&constructor_ref);
        let delete_ref = object::generate_delete_ref(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);

        move_to(&container_signer, TokenV1Container { token, delete_ref, transfer_ref });
        object::object_from_constructor_ref(&constructor_ref)
    }

    // Mutators

    /// This should be called at the end of a listing.
    public(friend) fun extract_or_transfer_tokenv1(
        closer: &signer,
        recipient: address,
        object: Object<TokenV1Container>,
    ) acquires TokenV1Container {
        let direct_transfer_enabled = tokenv1::get_direct_transfer(recipient);
        let object_addr = object::object_address(&object);
        if (direct_transfer_enabled) {
            let TokenV1Container {
                token,
                delete_ref,
                transfer_ref: _,
            } = move_from(object_addr);
            tokenv1::direct_deposit_with_opt_in(recipient, token);
            object::delete(delete_ref);
        } else if (signer::address_of(closer) == recipient) {
            let TokenV1Container {
                token,
                delete_ref,
                transfer_ref: _,
            } = move_from(object_addr);
            tokenv1::deposit_token(closer, token);
            object::delete(delete_ref);
        } else {
            let tokenv1_container = borrow_global<TokenV1Container>(object_addr);
            let linear_transfer_ref =
                object::generate_linear_transfer_ref(&tokenv1_container.transfer_ref);
            object::transfer_with_ref(linear_transfer_ref, recipient);
        };
    }

    /// If the account did not have tokenv1 enabled, then it must call this after making the
    /// purchase to extract the token.
    public entry fun extract_tokenv1(
        owner: &signer,
        object: Object<TokenV1Container>,
    ) acquires TokenV1Container {
        let object_addr = object::object_address(&object);
        assert!(
            object::is_owner(object, signer::address_of(owner)),
            error::permission_denied(ENOT_OWNER),
        );
        let TokenV1Container {
            token,
            delete_ref,
            transfer_ref: _,
        } = move_from(object_addr);
        object::delete(delete_ref);
        tokenv1::deposit_token(owner, token);
    }

    /// The listing has concluded, transfer the asset and delete the listing. Returns the seller
    /// for depositing any profit and the fee schedule for the marketplaces commission.
    public(friend) fun close(
        closer: &signer,
        object: Object<Listing>,
        recipient: address,
    ): (address, Object<FeeSchedule>) acquires Listing, TokenV1Container {
        let listing_addr = object::object_address(&object);
        let Listing {
            object,
            seller,
            fee_schedule,
            start_time: _,
            delete_ref,
            extend_ref,
        } = move_from<Listing>(listing_addr);

        let obj_signer = object::generate_signer_for_extending(&extend_ref);
        if (exists<TokenV1Container>(object::object_address(&object))) {
            extract_or_transfer_tokenv1(closer, recipient, object::convert(object));
        } else {
            object::transfer(&obj_signer, object, recipient);
        };
        object::delete(delete_ref);

        (seller, fee_schedule)
    }

    public(friend) fun assert_started(object: &Object<Listing>): address acquires Listing {
        let listing_addr = object::object_address(object);
        assert!(exists<Listing>(listing_addr), error::not_found(ENO_LISTING));

        let listing = borrow_global<Listing>(listing_addr);
        let now = timestamp::now_seconds();
        assert!(listing.start_time <= now, error::invalid_state(ELISTING_NOT_STARTED));
        listing_addr
    }

    // View

    #[view]
    public fun seller(object: Object<Listing>): address acquires Listing {
        let listing = borrow_listing(object);
        listing.seller
    }

    #[view]
    public fun listed_object(object: Object<Listing>): Object<ObjectCore> acquires Listing {
        let listing = borrow_listing(object);
        listing.object
    }

    #[view]
    public fun fee_schedule(object: Object<Listing>): Object<FeeSchedule> acquires Listing {
        let listing = borrow_listing(object);
        listing.fee_schedule
    }

    #[view]
    /// Compute the royalty either from the internal TokenV1, TokenV2 if it exists, or return
    /// no royalty.
    public fun compute_royalty(
        object: Object<Listing>,
        amount: u64,
    ): (address, u64) acquires Listing, TokenV1Container {
        let listing = borrow_listing(object);
        let obj_addr = object::object_address(&listing.object);
        if (exists<TokenV1Container>(obj_addr)) {
            let token_container = borrow_global<TokenV1Container>(obj_addr);
            let token_id = tokenv1::get_token_id(&token_container.token);
            let royalty = tokenv1::get_royalty(token_id);

            let payee_address = tokenv1::get_royalty_payee(&royalty);
            let numerator = tokenv1::get_royalty_numerator(&royalty);
            let denominator = tokenv1::get_royalty_denominator(&royalty);
            let royalty_amount = bounded_percentage(amount, numerator, denominator);
            (payee_address, royalty_amount)
        } else {
            let royalty = tokenv2::royalty(listing.object);
            if (option::is_some(&royalty)) {
                let royalty = option::destroy_some(royalty);
                let payee_address = royalty::payee_address(&royalty);
                let numerator = royalty::numerator(&royalty);
                let denominator = royalty::denominator(&royalty);

                let royalty_amount = bounded_percentage(amount, numerator, denominator);
                (payee_address, royalty_amount)
            } else {
                (@0x0, 0)
            }
        }
    }

    #[view]
    /// Produce a events::TokenMetadata for a listing
    public fun token_metadata(
        object: Object<Listing>,
    ): events::TokenMetadata acquires Listing, TokenV1Container {
        let listing = borrow_listing(object);
        let obj_addr = object::object_address(&listing.object);
        if (exists<TokenV1Container>(obj_addr)) {
            let token_container = borrow_global<TokenV1Container>(obj_addr);
            let token_id = tokenv1::get_token_id(&token_container.token);
            events::token_metadata_for_tokenv1(token_id)
        } else {
            events::token_metadata_for_tokenv2(object::convert(listing.object))
        }
    }

    /// Calculates a bounded percentage that can't go over 100% and handles 0 denominator as 0
    public inline fun bounded_percentage(amount: u64, numerator: u64, denominator: u64): u64 {
        if (denominator == 0) {
            0
        } else {
            math64::min(amount, math64::mul_div(amount, numerator, denominator))
        }
    }

    inline fun borrow_listing(object: Object<Listing>): &Listing acquires Listing {
        let obj_addr = object::object_address(&object);
        assert!(exists<Listing>(obj_addr), error::not_found(ENO_LISTING));
        borrow_global<Listing>(obj_addr)
    }
}
