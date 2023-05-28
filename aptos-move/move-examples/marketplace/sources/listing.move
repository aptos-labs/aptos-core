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

    use aptos_framework::object::{Self, ConstructorRef, DeleteRef, ExtendRef, Object, ObjectCore};
    use aptos_framework::timestamp;

    use aptos_token_objects::token as tokenv2;
    use aptos_token_objects::royalty;

    use marketplace::fee_schedule::FeeSchedule;

    /// There exists no listing.
    const ENO_LISTING: u64 = 1;
    /// The listing is not yet live.
    const ELISTING_NOT_STARTED: u64 = 2;
    /// The entity is not the creator.
    const ENOT_CREATOR: u64 = 3;

    // Core data structures

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
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

    // Mutators

    /// The listing has concluded, transfer the asset and delete the listing. Returns the seller
    /// for depositing any profit and the fee schedule for the marketplaces commission.
    public(friend) fun close(
        object: Object<Listing>,
        recipient: address,
    ): (address, Object<FeeSchedule>) acquires Listing {
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
        object::transfer(&obj_signer, object, recipient);
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
    public fun compute_royalty(object: Object<Listing>, amount: u64): (address, u64) acquires Listing {
        let listing = borrow_listing(object);
        let royalty = tokenv2::royalty(listing.object);
        if (option::is_some(&royalty)) {
            let royalty = option::destroy_some(royalty);
            let payee_address = royalty::payee_address(&royalty);
            let royalty_amount =
                (amount as u128) *
                (royalty::numerator(&royalty) as u128) /
                (royalty::denominator(&royalty) as u128);
            (payee_address, (royalty_amount as u64))
        } else {
            (@0x0, 0)
        }
    }

    inline fun borrow_listing(object: Object<Listing>): &Listing acquires Listing {
        let obj_addr = object::object_address(&object);
        assert!(exists<Listing>(obj_addr), error::not_found(ENO_LISTING));
        borrow_global<Listing>(obj_addr)
    }
}
