module open_marketplace::marketplace {
    use aptos_framework::object::{Self, ConstructorRef, Object, ObjectCore};
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::primary_fungible_store;

    use std::error;
    use std::signer;
    use aptos_std::math64;

    use open_marketplace::events;
    use open_marketplace::listing::{Self, Listing};
    use open_marketplace::fee_schedule::{Self, FeeSchedule};

    /// Error codes
    const ENO_LISTING: u64 = 1;
    const EINVALID_SELLER: u64 = 2;
    const EINVALID_PRICE: u64 = 3;
    const EBUYER_INSUFFICIENT_FUNDS: u64 = 4;
    const EINVALID_FA_TYPE: u64 = 5;
    

    // --- place_listing ---
    /// The owner of the NFT lists the NFT on the marketplace.
    public entry fun place_listing(
        seller: &signer,
        token_object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        fee_metadata: Object<Metadata>,
        price: u64,
    ) {
        // Check if the seller is the current owner of the token.
        assert!(object::owner(token_object) == signer::address_of(seller), error::permission_denied(EINVALID_SELLER));
        // Price validation now happens in listing::init

        init_place_listing_internal(
            seller,
            token_object,
            fee_schedule,
            price,
            fee_metadata
        );
    }

    public fun init_place_listing_internal(
        seller: &signer,
        token_object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        price: u64,
        fee_token_metadata: Object<Metadata>
    ): Object<Listing> {
        let constructor_ref = init(
            seller,
            token_object,
            fee_schedule,
            price,
            fee_token_metadata
        );

        // Get the listing object from the constructor ref
        let listing = object::object_from_constructor_ref<listing::Listing>(&constructor_ref);
        let listing_addr = object::object_address(&listing);
        
        // Emit the listing created event
        events::emit_listing_placed(
            fee_schedule,
            listing_addr,
            signer::address_of(seller),
            price,
            listing::fee_token_metadata(listing),
            listing::token_metadata(listing),
        );

        listing
    }
    

    inline fun init(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        price: u64,
        fee_token_metadata: Object<Metadata>
    ): ConstructorRef { 
        // We need to pass the fee_token_metadata explicitly since it's no longer available from fee_schedule
        listing::init(seller, object, fee_schedule, fee_token_metadata, price)
    }


    // --- delist ---
    /// The seller cancels their listing
    public entry fun cancel_listing(
        seller: &signer,
        listing_object: Object<Listing>
    ) {
        // assert that the listing exists
        let listing_addr = listing::assert_exists(&listing_object);

        // Verify the listing exists and has started
        let token_metadata = listing::token_metadata(listing_object);
        
        // Check that the caller is the seller
        let expected_seller_addr = signer::address_of(seller);
        let (actual_seller_addr, fee_schedule, fee_token_metadata, price) = listing::close(listing_object, expected_seller_addr);
        assert!(actual_seller_addr == expected_seller_addr, error::permission_denied(EINVALID_SELLER));
            
        events::emit_listing_canceled(
            fee_schedule,
            listing_addr,
            actual_seller_addr,
            price,
            fee_token_metadata,
            token_metadata,
        );
    }

    // --- purchase ---
    /// Purchase a listed NFT
    public entry fun fill_listing(
        purchaser: &signer,
        listing_object: Object<Listing>
    ) {
        // Verify the listing exists and has started
        let listing_addr = listing::assert_exists(&listing_object);
        
        let price = listing::price(listing_object);
        let seller_addr = listing::seller(listing_object);

        // Get the listing's fee token metadata
        let fa_metadata = listing::fee_token_metadata(listing_object);
        
        // Withdraw Fungible Asset from the purchaser's primary store
        let payment_fa = primary_fungible_store::withdraw(
            purchaser, // Withdraw from the purchaser's account
            fa_metadata, // The Metadata Object of the FA
            price // The amount to withdraw (listing price)
        );

        let total_amount_value = fungible_asset::amount(&payment_fa);

        // Get the fee schedule object using the provided view function
        let fee_schedule = listing::fee_schedule(listing_object);

        let (royalty_addr, royalty_charge) = listing::compute_royalty(listing_object, total_amount_value);
        let purchaser_addr = signer::address_of(purchaser);
        let commission_charge = fee_schedule::commission(fee_schedule, total_amount_value); // Use total_amount_value for commission calc


        // Extract royalty amount if applicable
        if (royalty_charge != 0) {
            let royalty_fa = fungible_asset::extract(&mut payment_fa, royalty_charge);
            primary_fungible_store::deposit(royalty_addr, royalty_fa);
        };

        // Extract commission amount
        if (commission_charge != 0) {
            // Ensure commission charged doesn't exceed remaining amount in payment_fa
             let actual_commission_charge = math64::min(fungible_asset::amount(&payment_fa), commission_charge);
             let commission_fa = fungible_asset::extract(&mut payment_fa, actual_commission_charge);
            // Deposit commission to the fee address's primary store
            primary_fungible_store::deposit(fee_schedule::fee_address(fee_schedule), commission_fa);
        };

        // Deposit remaining amount (the net amount after fees/royalties) to the seller
        // The remaining amount is still within the 'payment_fa' struct after extracts
        primary_fungible_store::deposit(seller_addr, payment_fa); // Deposit the rest to the seller
        let token_metadata = listing::token_metadata(listing_object);        

        let (_, _, listing_fee_token_metadata, _) = listing::close(listing_object, purchaser_addr); // Pass purchaser_addr as recipient

        events::emit_listing_filled(
            fee_schedule,
            listing_addr,
            seller_addr,
            purchaser_addr,
            price,
            listing_fee_token_metadata,
            commission_charge,
            royalty_charge,
            token_metadata,
        );
    }

    #[test_only]
    /// Expose fill_listing for testing
    public fun fill_listing_test(
        purchaser: &signer,
        listing_object: Object<Listing>
    ) {
        fill_listing(purchaser, listing_object)
    }
    
    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_token_objects::collection;
    #[test_only]
    use aptos_token_objects::token;
    #[test_only]
    use std::option;

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
        buyer: &signer,
        aptos_framework: &signer
    ) {
        let mint_cap = get_mint_cap(aptos_framework);
        coin::create_coin_conversion_map(aptos_framework);
        coin::create_pairing<AptosCoin>(aptos_framework);
        create_test_account(&mint_cap, creator);
        create_test_account(&mint_cap, buyer);
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

    #[test(creator = @0x123, buyer = @0x234, aptos_framework = @aptos_framework)]
    public fun test_fill_listing_fa_flow(
        creator: &signer,
        buyer: &signer,
        aptos_framework: &signer,
    ) {
        // Initialize test environment
        initialize(creator, buyer, aptos_framework);
        
        // Record initial balances
        let creator_addr = signer::address_of(creator);
        let buyer_addr = signer::address_of(buyer);
        
        // Create a fee schedule with 1% commission
        let fee_metadata = get_aptos_coin_metadata(creator);
        let fee_schedule_obj = fee_schedule::init_percentage(creator, creator_addr, 100, 1);
        
        // Create a test token
        let token_obj = create_test_token(creator);
        
        // Verify initial token ownership
        assert!(object::is_owner(token_obj, creator_addr), 0);
        
        // List the token
        let price = 500; // 500 units of AptosCoin
        let listing_obj = init_place_listing_internal(
            creator,
            token_obj,
            fee_schedule_obj,
            price,
            fee_metadata
        );
        
        // Get the correct FA metadata from the listing for the purchase
        let aptos_metadata = listing::fee_token_metadata(listing_obj);
        
        // Prepare fungible assets in the buyer's primary store
        let coin_amount = 1000; // More than the price
        let coin = coin::withdraw<AptosCoin>(buyer, coin_amount);
        let fa = coin::coin_to_fungible_asset(coin);
        primary_fungible_store::deposit(buyer_addr, fa);
        
        // Get balances before purchase
        let initial_buyer_fa_balance = primary_fungible_store::balance(buyer_addr, aptos_metadata);
        let initial_seller_fa_balance = primary_fungible_store::balance(creator_addr, aptos_metadata);
        
        // Buyer fills the listing
        fill_listing_test(buyer, listing_obj);
        
        // Verify token ownership transferred to buyer
        assert!(object::is_owner(token_obj, buyer_addr), 0);
        
        // Verify buyer's FA balance was reduced (at least by the price)
        let final_buyer_fa_balance = primary_fungible_store::balance(buyer_addr, aptos_metadata);
        assert!(final_buyer_fa_balance < initial_buyer_fa_balance, 0);
        
        // Verify seller received payment (minus commission)
        let final_seller_fa_balance = primary_fungible_store::balance(creator_addr, aptos_metadata);
        assert!(final_seller_fa_balance > initial_seller_fa_balance, 0);
    }

    #[test(creator = @0x123, buyer = @0x234, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10004, location = aptos_framework::fungible_asset)] // fungible_asset::EINSUFFICIENT_BALANCE
    public fun test_fill_listing_insufficient_balance(
        creator: &signer,
        buyer: &signer,
        aptos_framework: &signer,
    ) {
        initialize(creator, buyer, aptos_framework);
        
        let creator_addr = signer::address_of(creator);
        
        let fee_metadata = get_aptos_coin_metadata(creator);
        let fee_schedule_obj = fee_schedule::init_percentage(creator, creator_addr, 100, 1);
        
        let token_obj = create_test_token(creator);
        
        let price = DEFAULT_STARTING_BALANCE * 2; // Price higher than buyer's balance
        let listing_obj = init_place_listing_internal(
            creator,
            token_obj,
            fee_schedule_obj,
            price,
            fee_metadata
        );
        
        fill_listing_test(buyer, listing_obj);
    }

    #[test_only]
    // Define test coin structure
    struct TestCoin {}
}