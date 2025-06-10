module open_marketplace::listing {
    use aptos_framework::object::{Self, Object, ObjectCore, ConstructorRef, ExtendRef, DeleteRef};
    use aptos_framework::fungible_asset::{Metadata};
    use std::error;
    use std::signer;
    use std::option::{Self};
    use aptos_std::math64;

    use aptos_token_objects::token;
    use aptos_token_objects::royalty;
    
    use open_marketplace::events;
    use open_marketplace::fee_schedule::{Self, FeeSchedule};

    friend open_marketplace::marketplace;
    
    /// Error codes
    const ENO_LISTING: u64 = 1;
    const ELISTING_NOT_STARTED: u64 = 2;
    const ENOT_OWNER: u64 = 3;
    const EINVALID_PRICE: u64 = 4;
    const EZERO_SELLER_PROFIT: u64 = 5;

    // Resource that stores the listing information
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Listing has key {
        /// The object being sold (NFT)
        object: Object<ObjectCore>,
        /// Address of the seller
        seller: address,
        /// Price of the listing
        price: u64,
        /// Fee schedule for the listing
        fee_schedule: Object<FeeSchedule>,
        /// The Metadata Object of the Fungible Asset used for listing price
        fee_token_metadata: Object<Metadata>,
        /// DeleteRef to delete the listing object
        delete_ref: DeleteRef,
        /// There is no support for generating a signer with a TransferRef to transfer the listing object so using ExtendRef
        extend_ref: ExtendRef,
    }

    /// Initialize a new listing
    public fun init(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        fee_token_metadata: Object<Metadata>,
        price: u64
    ): ConstructorRef {
        // Verify the price is greater than zero
        assert!(price > 0, error::invalid_argument(EINVALID_PRICE));
        
        // Calculate the expected commission based on price and fee schedule
        let commission_charge = fee_schedule::commission(fee_schedule, price);
        
        // Calculate potential royalty on the token
        let royalty = token::royalty(object);
        let royalty_charge = if (option::is_some(&royalty)) {
            let royalty = option::borrow(&royalty);
            let numerator = royalty::numerator(royalty);
            let denominator = royalty::denominator(royalty);
            bounded_percentage(price, numerator, denominator)
        } else {
            0
        };

        // Ensure the seller will receive a profit (price > commission + royalty)
        assert!(
            price > (commission_charge + royalty_charge),
            error::invalid_argument(EZERO_SELLER_PROFIT)
        );
        
        // Create a new object to represent the listing
        let constructor_ref = object::create_object_from_account(seller);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::disable_ungated_transfer(&transfer_ref);
        
        // Get a signer for the newly created object
        let listing_signer = object::generate_signer(&constructor_ref);
        
        // Create and move the Listing resource to the new object
        move_to(&listing_signer, Listing {
            object,
            seller: signer::address_of(seller),
            price,
            fee_schedule,
            fee_token_metadata,
            delete_ref: object::generate_delete_ref(&constructor_ref),
            extend_ref: object::generate_extend_ref(&constructor_ref),
        });
        
        // Get the address of the listing object
        // Note: listing_addr and signer::address_of(listing_signer) both reference the same address
        // - listing_addr is used for operations requiring just the address
        // - listing_signer is used for operations requiring authorization (like move_to above)
        let listing_addr = object::address_from_constructor_ref(&constructor_ref);
        object::transfer(seller, object, listing_addr);
        
        constructor_ref
    }

    /// Close a listing and transfer the object to the recipient
    public fun close(
        object: Object<Listing>,
        recipient: address
    ): (address, Object<FeeSchedule>, Object<Metadata>, u64) acquires Listing {
        let listing_addr = object::object_address(&object);
        let Listing {
            object: listed_object,
            seller,
            price,
            fee_schedule,
            fee_token_metadata,
            delete_ref,
            extend_ref,
        } = move_from<Listing>(listing_addr);

        // Transfer the listed object to the recipient
        let obj_signer = object::generate_signer_for_extending(&extend_ref);
        object::transfer(&obj_signer, listed_object, recipient);
        object::delete(delete_ref);

        (seller, fee_schedule, fee_token_metadata, price)
    }

    /// Validate that a listing exists 
    public fun assert_exists(object: &Object<Listing>): address {
        let listing_addr = object::object_address(object);
        assert!(exists<Listing>(listing_addr), error::not_found(ENO_LISTING));
        listing_addr
    }

    //
    // View Functions
    //

    #[view]
    /// Produce a events::TokenMetadata for a listing
    public fun token_metadata(
        object: Object<Listing>,
    ): events::TokenMetadata acquires Listing {
        let listing = borrow_listing(object);
        events::token_metadata(object::convert(listing.object))
    }

    #[view]
    /// Compute the royalty from Token if it exists, or return no royalty
    public fun compute_royalty(
        object: Object<Listing>,
        amount: u64,
    ): (address, u64) acquires Listing {
        let listing = borrow_listing(object);
        let royalty = token::royalty(listing.object);
        
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

    /// Calculates a bounded percentage that can't go over 100% and handles 0 denominator as 0
    inline fun bounded_percentage(amount: u64, numerator: u64, denominator: u64): u64 {
        if (denominator == 0) {
            0
        } else {
            math64::min(amount, math64::mul_div(amount, numerator, denominator))
        }
    }

    /// Helper to borrow a listing
    public(friend) inline fun borrow_listing(object: Object<Listing>): &Listing acquires Listing {
        let obj_addr = object::object_address(&object);
        assert!(exists<Listing>(obj_addr), error::not_found(ENO_LISTING));
        borrow_global<Listing>(obj_addr)
    }

    #[view]
    /// Get the seller of a listing
    public fun seller(object: Object<Listing>): address acquires Listing {
        let listing = borrow_listing(object);
        listing.seller
    }

    #[view]
    /// Get the listed object
    public fun listed_object(object: Object<Listing>): Object<ObjectCore> acquires Listing {
        let listing = borrow_listing(object);
        listing.object
    }

    #[view]
    /// Get the price of a listing
    public fun price(object: Object<Listing>): u64 acquires Listing {
        let listing = borrow_listing(object);
        listing.price
    }

    #[view]
    /// Get the fee_schedule of a listing
    public fun fee_schedule(object: Object<Listing>): Object<FeeSchedule> acquires Listing {
        let listing = borrow_listing(object);
        listing.fee_schedule
    }

    #[view]
    /// Get the fee_token_metadata of a listing
    public fun fee_token_metadata(object: Object<Listing>): Object<Metadata> acquires Listing {
        let listing = borrow_listing(object);
        listing.fee_token_metadata
    }

    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::primary_fungible_store;
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_token_objects::collection;
    #[test_only]
    use aptos_framework::fungible_asset;

    #[test_only]
    const DEFAULT_STARTING_BALANCE: u64 = 10000;

    #[test_only]
    /// Helper function to get a mint capability for AptosCoin
    fun get_mint_cap(aptos_framework: &signer): coin::MintCapability<AptosCoin> {
        let (burn_cap, freeze_cap, mint_cap) =
            coin::initialize<AptosCoin>(
                aptos_framework,
                string::utf8(b"TC"),
                string::utf8(b"TC"),
                8,
                false
            );
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_burn_cap(burn_cap);
        mint_cap
    }

    #[test_only]
    /// Create a test account with AptosCoin balance
    fun create_test_account(
        mint_cap: &coin::MintCapability<AptosCoin>, account: &signer
    ) {
        account::create_account_for_test(signer::address_of(account));
        coin::register<AptosCoin>(account);
        let coins = coin::mint<AptosCoin>(DEFAULT_STARTING_BALANCE, mint_cap);
        coin::deposit(signer::address_of(account), coins);
    }

    #[test_only]
    /// Initialize test environment
    fun initialize(
        creator: &signer,
        aptos_framework: &signer
    ) {
        let mint_cap = get_mint_cap(aptos_framework);
        coin::create_coin_conversion_map(aptos_framework);
        coin::create_pairing<AptosCoin>(aptos_framework);
        create_test_account(&mint_cap, creator);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test_only]
    /// Helper function to create a token for testing
    fun create_test_token(creator: &signer): Object<ObjectCore> {
        let collection_name = string::utf8(b"Test Collection");
        let token_name = string::utf8(b"Test Token");

        // Create collection
        let _collection_constructor_ref = collection::create_unlimited_collection(
            creator,
            string::utf8(b"Collection description"),
            collection_name,
            option::none(),
            string::utf8(b"Collection URI"),
        );

        // Create token with no royalty
        let token_constructor_ref = token::create(
            creator,
            collection_name,
            string::utf8(b"Token description"),
            token_name,
            option::none(),
            string::utf8(b"Token URI"),
        );

        object::object_from_constructor_ref<ObjectCore>(&token_constructor_ref)
    }

    #[test_only]
    /// Helper function to get FA metadata from AptosCoin
    fun get_aptos_coin_metadata(creator: &signer): Object<Metadata> {
        let creator_addr = signer::address_of(creator);
        
        // Create FA metadata from real AptosCoin
        let coin = coin::withdraw<AptosCoin>(creator, 1000);
        let fa = coin::coin_to_fungible_asset(coin);
        let fa_metadata = fungible_asset::metadata_from_asset(&fa);
        
        // Deposit the fungible asset back to avoid the drop error
        primary_fungible_store::deposit(creator_addr, fa);
        
        fa_metadata
    }

    // Tests

    #[test(creator = @0x123, buyer = @0x234, aptos_framework = @aptos_framework)]
    public fun test_listing_create_and_close(
        creator: &signer,
        buyer: &signer,
        aptos_framework: &signer,
    ) acquires Listing {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 1);
        
        let token_obj = create_test_token(creator);
        
        let price = 500;
        let constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
        let listing_obj = object::object_from_constructor_ref<Listing>(&constructor_ref);
        
        // Verify the listing was created correctly
        assert!(exists<Listing>(object::object_address(&listing_obj)), 0);
        
        // Verify seller, price, and fee_schedule
        assert!(seller(listing_obj) == signer::address_of(creator), 0);
        assert!(price(listing_obj) == price, 0);
        assert!(object::object_address(&fee_schedule(listing_obj)) == object::object_address(&fee_schedule_obj), 0);
        assert!(object::object_address(&fee_token_metadata(listing_obj)) == object::object_address(&fee_metadata), 0);
        
        // Close the listing and transfer to buyer
        let buyer_addr = signer::address_of(buyer);
        let (seller_addr, fs, ftm, sale_price) = close(listing_obj, buyer_addr);
        
        // Verify the returned values
        assert!(seller_addr == signer::address_of(creator), 0);
        assert!(object::object_address(&fs) == object::object_address(&fee_schedule_obj), 0);
        assert!(object::object_address(&ftm) == object::object_address(&fee_metadata), 0);
        assert!(sale_price == price, 0);
        
        // Verify the listing no longer exists and the token is now owned by the buyer
        assert!(!exists<Listing>(object::object_address(&listing_obj)), 0);    
        assert!(object::is_owner(token_obj, buyer_addr), 0);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    public fun test_token_metadata_and_royalty(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires Listing {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 1);        
        let token_obj = create_test_token(creator);
        
        let price = 1000; 
        let constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
        let listing_obj = object::object_from_constructor_ref<Listing>(&constructor_ref);
        
        // Verify the token metadata can be retrieved
        let listed_obj = listed_object(listing_obj);
        assert!(object::object_address(&listed_obj) == object::object_address(&token_obj), 0);
        
        // Test royalty calculation (with no royalty configured)
        let (royalty_addr, royalty_amount) = compute_royalty(listing_obj, price);
        assert!(royalty_addr == @0x0, 0); 
        assert!(royalty_amount == 0, 0);  
        
        // Test bounded percentage calculation
        assert!(bounded_percentage(100, 10, 100) == 10, 0); // 10%
        assert!(bounded_percentage(100, 10, 0) == 0, 0);    // Denominator 0 should return 0
        assert!(bounded_percentage(100, 200, 100) == 100, 0); // Capped at maximum amount
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10005, location = Self)] // error::invalid_argument(EZERO_SELLER_PROFIT)
    public fun test_create_listing_with_zero_seller_profit(
        creator: &signer,
        aptos_framework: &signer,
    ) {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        
        // Create a fee schedule with 100% commission (this alone would cause zero profit)
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 100);
        
        let token_obj = create_test_token(creator);
        
        let price = 500;
        // This should fail because the commission would be 100% of the price
        let _constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10004, location = Self)] // error::invalid_argument(EINVALID_PRICE)
    public fun test_create_listing_with_zero_price(
        creator: &signer,
        aptos_framework: &signer,
    ) {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 1);
        
        let token_obj = create_test_token(creator);
        
        let price = 0; // Zero price should be rejected
        let _constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10005, location = Self)] // error::invalid_argument(EZERO_SELLER_PROFIT)
    public fun test_create_listing_with_royalty_and_commission_exceeding_price(
        creator: &signer,
        aptos_framework: &signer,
    ) {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        
        // Create a fee schedule with 60% commission 
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 60);
        
        // Create a token with 50% royalty
        let collection_name = string::utf8(b"Test Collection");
        let token_name = string::utf8(b"Test Token");

        // Create collection
        let _collection_constructor_ref = collection::create_unlimited_collection(
            creator,
            string::utf8(b"Collection description"),
            collection_name,
            option::none(),
            string::utf8(b"Collection URI"),
        );

        // Create royalty
        let royalty = royalty::create(50, 100, signer::address_of(creator));
        
        // Create token with royalty
        let token_constructor_ref = token::create(
            creator,
            collection_name,
            string::utf8(b"Token description"),
            token_name,
            option::some(royalty),
            string::utf8(b"Token URI"),
        );

        let token_obj = object::object_from_constructor_ref<ObjectCore>(&token_constructor_ref);
        
        let price = 1000;
        // This should fail because the commission (60%) + royalty (50%) = 110% > 100%
        let _constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10005, location = Self)] // error::invalid_argument(EZERO_SELLER_PROFIT)
    public fun test_create_listing_with_royalty_and_commission_equal_to_price(
        creator: &signer,
        aptos_framework: &signer,
    ) {
        initialize(creator, aptos_framework);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        
        // Create a fee schedule with 50% commission 
        let fee_schedule_obj = fee_schedule::init_percentage(creator, signer::address_of(creator), 100, 50);
        
        // Create a token with 50% royalty
        let collection_name = string::utf8(b"Test Collection");
        let token_name = string::utf8(b"Test Token");

        // Create collection
        let _collection_constructor_ref = collection::create_unlimited_collection(
            creator,
            string::utf8(b"Collection description"),
            collection_name,
            option::none(),
            string::utf8(b"Collection URI"),
        );

        // Create royalty
        let royalty = royalty::create(50, 100, signer::address_of(creator));
        
        // Create token with royalty
        let token_constructor_ref = token::create(
            creator,
            collection_name,
            string::utf8(b"Token description"),
            token_name,
            option::some(royalty),
            string::utf8(b"Token URI"),
        );

        let token_obj = object::object_from_constructor_ref<ObjectCore>(&token_constructor_ref);
        
        let price = 1000;
        // This should fail because the commission (50%) + royalty (50%) = 100% which equals price
        // Our check requires price > (commission + royalty), so no scenario where they're equal
        let _constructor_ref = init(creator, token_obj, fee_schedule_obj, fee_metadata, price);
    }

}
