address marketplace {
/// Defines a single listing or an item for sale or auction. This is an escrow service that
/// enables two parties to exchange one asset for another.
/// Each listing has the following properties:
/// * FeeSchedule specifying payment flows
/// * Owner or the person that can end the sale or auction
/// * Optional buy it now price
/// * Ending time at which point it can be claimed by the highest bidder or left in escrow.
/// * For auctions, the minimum bid rate and optional increase in duration of the auction if bids
///   are made toward the end of the auction.
module coin_listing {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use velor_std::math64;

    use velor_framework::coin::{Self, Coin};
    use velor_framework::object::{Self, ConstructorRef, Object, ObjectCore};
    use velor_framework::timestamp;

    use marketplace::events;
    use marketplace::fee_schedule::{Self, FeeSchedule};
    use marketplace::listing::{Self, Listing};
    use velor_framework::velor_account;

    #[test_only]
    friend marketplace::listing_tests;

    /// There exists no listing.
    const ENO_LISTING: u64 = 1;
    /// This is an auction without buy it now.
    const ENO_BUY_IT_NOW: u64 = 2;
    /// The proposed bid is insufficient.
    const EBID_TOO_LOW: u64 = 3;
    /// The auction has not yet ended.
    const EAUCTION_NOT_ENDED: u64 = 4;
    /// The auction has already ended.
    const EAUCTION_ENDED: u64 = 5;
    /// The entity is not the seller.
    const ENOT_SELLER: u64 = 6;

    // Core data structures
    const FIXED_PRICE_TYPE: vector<u8> = b"fixed price";
    const AUCTION_TYPE: vector<u8> = b"auction";

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Fixed-price market place listing.
    struct FixedPriceListing<phantom CoinType> has key {
        /// The price to purchase the item up for listing.
        price: u64,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// An auction-based listing with optional buy it now semantics.
    struct AuctionListing<phantom CoinType> has key {
        /// Starting bid price.
        starting_bid: u64,
        /// Price increment from the current bid.
        bid_increment: u64,
        /// Current bid, if one exists.
        current_bid: Option<Bid<CoinType>>,
        /// Auction end time in Unix time as seconds.
        auction_end_time: u64,
        /// If a bid time comes within this amount of time before the end bid, extend the end bid
        /// to the current time plus this amount.
        minimum_bid_time_before_end: u64,
        /// Buy it now price, ends auction immediately.
        buy_it_now_price: Option<u64>,
    }

    /// Represents a single bid within this auction house.
    struct Bid<phantom CoinType> has store {
        bidder: address,
        coins: Coin<CoinType>,
    }

    // Init functions

    public entry fun init_fixed_price<CoinType>(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        price: u64,
    ) {
        init_fixed_price_internal<CoinType>(seller, object, fee_schedule, start_time, price);
    }

    public(friend) fun init_fixed_price_internal<CoinType>(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        price: u64,
    ): Object<Listing> {
        let (listing_signer, constructor_ref) = init<CoinType>(
            seller,
            object,
            fee_schedule,
            start_time,
            price,
        );

        let fixed_price_listing = FixedPriceListing<CoinType> {
            price,
        };
        move_to(&listing_signer, fixed_price_listing);

        let listing = object::object_from_constructor_ref(&constructor_ref);

        events::emit_listing_placed(
            fee_schedule,
            string::utf8(FIXED_PRICE_TYPE),
            object::object_address(&listing),
            signer::address_of(seller),
            price,
            listing::token_metadata(listing),
        );

        listing
    }

    public entry fun init_fixed_price_for_tokenv1<CoinType>(
        seller: &signer,
        token_creator: address,
        token_collection: String,
        token_name: String,
        token_property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        price: u64,
    ) {
        init_fixed_price_for_tokenv1_internal<CoinType>(
            seller,
            token_creator,
            token_collection,
            token_name,
            token_property_version,
            fee_schedule,
            start_time,
            price,
        );
    }

    public(friend) fun init_fixed_price_for_tokenv1_internal<CoinType>(
        seller: &signer,
        token_creator: address,
        token_collection: String,
        token_name: String,
        token_property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        price: u64,
    ): Object<Listing> {
        let object = listing::create_tokenv1_container(
            seller,
            token_creator,
            token_collection,
            token_name,
            token_property_version,
        );
        init_fixed_price_internal<CoinType>(
            seller,
            object::convert(object),
            fee_schedule,
            start_time,
            price,
        )
    }

    public entry fun init_auction<CoinType>(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        starting_bid: u64,
        bid_increment: u64,
        auction_end_time: u64,
        minimum_bid_time_before_end: u64,
        buy_it_now_price: Option<u64>,
    ) {
        init_auction_internal<CoinType>(
            seller,
            object,
            fee_schedule,
            start_time,
            starting_bid,
            bid_increment,
            auction_end_time,
            minimum_bid_time_before_end,
            buy_it_now_price,
        );
    }

    public(friend) fun init_auction_internal<CoinType>(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        starting_bid: u64,
        bid_increment: u64,
        auction_end_time: u64,
        minimum_bid_time_before_end: u64,
        buy_it_now_price: Option<u64>,
    ): Object<Listing> {
        let (listing_signer, constructor_ref) = init<CoinType>(
            seller,
            object,
            fee_schedule,
            start_time,
            starting_bid,
        );

        let auction_listing = AuctionListing<CoinType> {
            starting_bid,
            bid_increment,
            current_bid: option::none(),
            auction_end_time,
            minimum_bid_time_before_end,
            buy_it_now_price,
        };
        move_to(&listing_signer, auction_listing);
        let listing = object::object_from_constructor_ref(&constructor_ref);

        events::emit_listing_placed(
            fee_schedule,
            string::utf8(AUCTION_TYPE),
            object::object_address(&listing),
            signer::address_of(seller),
            starting_bid,
            listing::token_metadata(listing),
        );

        listing
    }

    public entry fun init_auction_for_tokenv1<CoinType>(
        seller: &signer,
        token_creator: address,
        token_collection: String,
        token_name: String,
        token_property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        starting_bid: u64,
        bid_increment: u64,
        auction_end_time: u64,
        minimum_bid_time_before_end: u64,
        buy_it_now_price: Option<u64>,
    ) {
        init_auction_for_tokenv1_internal<CoinType>(
            seller,
            token_creator,
            token_collection,
            token_name,
            token_property_version,
            fee_schedule,
            start_time,
            starting_bid,
            bid_increment,
            auction_end_time,
            minimum_bid_time_before_end,
            buy_it_now_price,
        );
    }

    public(friend) fun init_auction_for_tokenv1_internal<CoinType>(
        seller: &signer,
        token_creator: address,
        token_collection: String,
        token_name: String,
        token_property_version: u64,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        starting_bid: u64,
        bid_increment: u64,
        auction_end_time: u64,
        minimum_bid_time_before_end: u64,
        buy_it_now_price: Option<u64>,
    ): Object<Listing> {
        let object = listing::create_tokenv1_container(
            seller,
            token_creator,
            token_collection,
            token_name,
            token_property_version,
        );
        init_auction_internal<CoinType>(
            seller,
            object::convert(object),
            fee_schedule,
            start_time,
            starting_bid,
            bid_increment,
            auction_end_time,
            minimum_bid_time_before_end,
            buy_it_now_price,
        )
    }

    inline fun init<CoinType>(
        seller: &signer,
        object: Object<ObjectCore>,
        fee_schedule: Object<FeeSchedule>,
        start_time: u64,
        initial_price: u64,
    ): (signer, ConstructorRef) {
        velor_account::transfer_coins<CoinType>(
            seller,
            fee_schedule::fee_address(fee_schedule),
            fee_schedule::listing_fee(fee_schedule, initial_price),
        );

        listing::init(seller, object, fee_schedule, start_time)
    }

    // Mutators

    /// Purchase outright an item from an auction or a fixed price listing.
    public entry fun purchase<CoinType>(
        purchaser: &signer,
        object: Object<Listing>,
    ) acquires AuctionListing, FixedPriceListing {
        let listing_addr = listing::assert_started(&object);

        // Retrieve the purchase price if the auction has buy it now or this is a fixed listing.
        let (price, type) = if (exists<AuctionListing<CoinType>>(listing_addr)) {
            let AuctionListing {
                starting_bid: _,
                bid_increment: _,
                current_bid,
                auction_end_time,
                minimum_bid_time_before_end: _,
                buy_it_now_price,
            } = move_from<AuctionListing<CoinType>>(listing_addr);

            let now = timestamp::now_seconds();
            assert!(now < auction_end_time, error::invalid_state(EAUCTION_ENDED));

            assert!(option::is_some(&buy_it_now_price), error::invalid_argument(ENO_BUY_IT_NOW));
            if (option::is_some(&current_bid)) {
                let Bid { bidder, coins } = option::destroy_some(current_bid);
                velor_account::deposit_coins(bidder, coins);
            } else {
                option::destroy_none(current_bid);
            };
            (option::destroy_some(buy_it_now_price), string::utf8(AUCTION_TYPE))
        } else if (exists<FixedPriceListing<CoinType>>(listing_addr)) {
            let FixedPriceListing {
                price,
            } = move_from<FixedPriceListing<CoinType>>(listing_addr);
            (price, string::utf8(FIXED_PRICE_TYPE))
        } else {
            // This should just be an abort but the compiler errors.
            abort (error::not_found(ENO_LISTING))
        };

        let coins = coin::withdraw<CoinType>(purchaser, price);

        complete_purchase(purchaser, signer::address_of(purchaser), object, coins, type)
    }

    /// End a fixed price listing early.
    public entry fun end_fixed_price<CoinType>(
        seller: &signer,
        object: Object<Listing>,
    ) acquires FixedPriceListing {
        let token_metadata = listing::token_metadata(object);

        let expected_seller_addr = signer::address_of(seller);
        let (actual_seller_addr, fee_schedule) = listing::close(seller, object, expected_seller_addr);
        assert!(expected_seller_addr == actual_seller_addr, error::permission_denied(ENOT_SELLER));

        let listing_addr = object::object_address(&object);
        assert!(exists<FixedPriceListing<CoinType>>(listing_addr), error::not_found(ENO_LISTING));
        let FixedPriceListing {
            price,
        } = move_from<FixedPriceListing<CoinType>>(listing_addr);

        events::emit_listing_canceled(
            fee_schedule,
            string::utf8(FIXED_PRICE_TYPE),
            listing_addr,
            actual_seller_addr,
            price,
            token_metadata,
        );
    }

    /// Make a bid on a listing. If the listing comes in near the end of an auction, the auction
    /// may be extended to give at least minimum_bid_time_before_end time remaining in the auction.
    public entry fun bid<CoinType>(
        bidder: &signer,
        object: Object<Listing>,
        bid_amount: u64,
    ) acquires AuctionListing {
        let listing_addr = listing::assert_started(&object);
        assert!(exists<AuctionListing<CoinType>>(listing_addr), error::not_found(ENO_LISTING));
        let auction_listing = borrow_global_mut<AuctionListing<CoinType>>(listing_addr);

        let now = timestamp::now_seconds();
        assert!(now < auction_listing.auction_end_time, error::invalid_state(EAUCTION_ENDED));

        let (previous_bidder, previous_bid, minimum_bid) = if (option::is_some(&auction_listing.current_bid)) {
            let Bid { bidder, coins } = option::extract(&mut auction_listing.current_bid);
            let current_bid = coin::value(&coins);
            velor_account::deposit_coins(bidder, coins);
            (option::some(bidder), option::some(current_bid), current_bid + auction_listing.bid_increment)
        } else {
            (option::none(), option::none(), auction_listing.starting_bid)
        };

        assert!(bid_amount >= minimum_bid, error::invalid_argument(EBID_TOO_LOW));
        let coins = coin::withdraw<CoinType>(bidder, bid_amount);
        let bid = Bid {
            bidder: signer::address_of(bidder),
            coins,
        };
        option::fill(&mut auction_listing.current_bid, bid);

        let fee_schedule = listing::fee_schedule(object);
        velor_account::transfer_coins<CoinType>(
            bidder,
            fee_schedule::fee_address(fee_schedule),
            fee_schedule::bidding_fee(fee_schedule, bid_amount),
        );

        let now = timestamp::now_seconds();
        let current_end_time = auction_listing.auction_end_time;
        let minimum_end_time = now + auction_listing.minimum_bid_time_before_end;

        if (current_end_time < minimum_end_time) {
            auction_listing.auction_end_time = minimum_end_time
        };

        events::emit_bid_event(
            fee_schedule,
            listing_addr,
            signer::address_of(bidder),
            bid_amount,
            auction_listing.auction_end_time,
            previous_bidder,
            previous_bid,
            current_end_time,
            listing::token_metadata(object),
        );
    }

    /// Once the current time has elapsed the auctions run time, allow the auction to be settled by
    /// distributing out the asset to the winner or the auction seller if no one bid as well as
    /// giving any fees to the marketplace that hosted the auction.
    public entry fun complete_auction<CoinType>(
        completer: &signer,
        object: Object<Listing>,
    ) acquires AuctionListing {
        let listing_addr = listing::assert_started(&object);
        assert!(exists<AuctionListing<CoinType>>(listing_addr), error::not_found(ENO_LISTING));

        let AuctionListing {
            starting_bid: _,
            bid_increment: _,
            current_bid,
            auction_end_time,
            minimum_bid_time_before_end: _,
            buy_it_now_price: _,
        } = move_from<AuctionListing<CoinType>>(listing_addr);

        let now = timestamp::now_seconds();
        assert!(auction_end_time <= now, error::invalid_state(EAUCTION_NOT_ENDED));

        let seller = listing::seller(object);

        let (purchaser, coins) = if (option::is_some(&current_bid)) {
            let Bid { bidder, coins } = option::destroy_some(current_bid);
            (bidder, coins)
        } else {
            option::destroy_none(current_bid);
            (seller, coin::zero<CoinType>())
        };

        complete_purchase(completer, purchaser, object, coins, string::utf8(AUCTION_TYPE));
    }

    inline fun complete_purchase<CoinType>(
        completer: &signer,
        purchaser_addr: address,
        object: Object<Listing>,
        coins: Coin<CoinType>,
        type: String,
    ) {
        let token_metadata = listing::token_metadata(object);

        let price = coin::value(&coins);
        let (royalty_addr, royalty_charge) = listing::compute_royalty(object, price);
        let (seller, fee_schedule) = listing::close(completer, object, purchaser_addr);

        // Take royalty first
        if (royalty_charge != 0) {
            let royalty = coin::extract(&mut coins, royalty_charge);
            velor_account::deposit_coins(royalty_addr, royalty);
        };

        // Take commission of what's left, creators get paid first
        let commission_charge = fee_schedule::commission(fee_schedule, price);
        let actual_commission_charge = math64::min(coin::value(&coins), commission_charge);
        let commission = coin::extract(&mut coins, actual_commission_charge);
        velor_account::deposit_coins(fee_schedule::fee_address(fee_schedule), commission);

        // Seller gets what is left
        velor_account::deposit_coins(seller, coins);

        events::emit_listing_filled(
            fee_schedule,
            type,
            object::object_address(&object),
            seller,
            purchaser_addr,
            price,
            commission_charge,
            royalty_charge,
            token_metadata,
        );
    }

    // View

    #[view]
    public fun price<CoinType>(
        object: Object<Listing>,
    ): Option<u64> acquires AuctionListing, FixedPriceListing {
        let listing_addr = object::object_address(&object);
        if (exists<FixedPriceListing<CoinType>>(listing_addr)) {
            let fixed_price = borrow_global<FixedPriceListing<CoinType>>(listing_addr);
            option::some(fixed_price.price)
        } else if (exists<AuctionListing<CoinType>>(listing_addr)) {
            borrow_global<AuctionListing<CoinType>>(listing_addr).buy_it_now_price
        } else {
            // This should just be an abort but the compiler errors.
            assert!(false, error::not_found(ENO_LISTING));
            option::none()
        }
    }

    #[view]
    public fun is_auction<CoinType>(object: Object<Listing>): bool {
        let obj_addr = object::object_address(&object);
        exists<AuctionListing<CoinType>>(obj_addr)
    }

    #[view]
    public fun starting_bid<CoinType>(object: Object<Listing>): u64 acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        auction.starting_bid
    }

    #[view]
    public fun bid_increment<CoinType>(object: Object<Listing>): u64 acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        auction.bid_increment
    }

    #[view]
    public fun auction_end_time<CoinType>(object: Object<Listing>): u64 acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        auction.auction_end_time
    }

    #[view]
    public fun minimum_bid_time_before_end<CoinType>(
        object: Object<Listing>,
    ): u64 acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        auction.minimum_bid_time_before_end
    }

    #[view]
    public fun current_bidder<CoinType>(
        object: Object<Listing>,
    ): Option<address> acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        if (option::is_some(&auction.current_bid)) {
            option::some(option::borrow(&auction.current_bid).bidder)
        } else {
            option::none()
        }
    }

    #[view]
    public fun current_amount<CoinType>(
        object: Object<Listing>,
    ): Option<u64> acquires AuctionListing {
        let auction = borrow_auction<CoinType>(object);
        if (option::is_some(&auction.current_bid)) {
            let coins = &option::borrow(&auction.current_bid).coins;
            option::some(coin::value(coins))
        } else {
            option::none()
        }
    }

    inline fun borrow_auction<CoinType>(
        object: Object<Listing>,
    ): &AuctionListing<CoinType> acquires AuctionListing {
        let obj_addr = object::object_address(&object);
        assert!(exists<AuctionListing<CoinType>>(obj_addr), error::not_found(ENO_LISTING));
        borrow_global<AuctionListing<CoinType>>(obj_addr)
    }

    inline fun borrow_fixed_price<CoinType>(
        object: Object<Listing>,
    ): &FixedPriceListing<CoinType> acquires FixedPriceListing {
        let obj_addr = object::object_address(&object);
        assert!(exists<FixedPriceListing<CoinType>>(obj_addr), error::not_found(ENO_LISTING));
        borrow_global<FixedPriceListing<CoinType>>(obj_addr)
    }
}

// Tests

#[test_only]
module listing_tests {
    use std::option;

    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::coin;
    use velor_framework::object::{Self, Object};
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;

    use velor_token_objects::token::Token;
    use marketplace::test_utils::{mint_tokenv2_with_collection_royalty, mint_tokenv1_additional_royalty, mint_tokenv1};

    use marketplace::coin_listing;
    use marketplace::fee_schedule::FeeSchedule;
    use marketplace::listing::{Self, Listing};
    use marketplace::test_utils;

    fun test_fixed_price(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, fee_schedule, listing) = fixed_price_listing(marketplace, seller);

        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
        assert!(listing::listed_object(listing) == object::convert(token), 0);
        assert!(listing::fee_schedule(listing) == fee_schedule, 0);
        assert!(coin_listing::price<VelorCoin>(listing) == option::some(500), 0);
        assert!(!coin_listing::is_auction<VelorCoin>(listing), 0);

        coin_listing::purchase<VelorCoin>(purchaser, listing);

        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10494, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9500, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_high_royalty(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        // TODO: add test that separates seller and creator
        let (_collection, additional_token) = mint_tokenv2_with_collection_royalty(seller, 100, 100);
        let (token, fee_schedule, listing) = fixed_price_listing_with_token(marketplace, seller, additional_token);

        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
        assert!(listing::listed_object(listing) == object::convert(token), 0);
        assert!(listing::fee_schedule(listing) == fee_schedule, 0);
        assert!(coin_listing::price<VelorCoin>(listing) == option::some(500), 0);
        assert!(!coin_listing::is_auction<VelorCoin>(listing), 0);

        coin_listing::purchase<VelorCoin>(purchaser, listing);

        assert!(object::owner(token) == purchaser_addr, 0);
        // Because royalty is 100, no commission is taken just the listing fee
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10499, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9500, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_end(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, _purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);

        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        coin_listing::end_fixed_price<VelorCoin>(seller, listing);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
        assert!(object::owner(token) == seller_addr, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
        assert!(listing::listed_object(listing) == object::convert(token), 0);
        assert!(listing::fee_schedule(listing) == fee_schedule, 0);
        assert!(coin_listing::price<VelorCoin>(listing) == option::some(500), 0);
        assert!(coin_listing::is_auction<VelorCoin>(listing), 0);
        assert!(coin_listing::starting_bid<VelorCoin>(listing) == 100, 0);
        assert!(coin_listing::bid_increment<VelorCoin>(listing) == 50, 0);
        assert!(coin_listing::auction_end_time<VelorCoin>(listing) == timestamp::now_seconds() + 200, 0);
        assert!(coin_listing::minimum_bid_time_before_end<VelorCoin>(listing) == 150, 0);
        assert!(coin_listing::current_amount<VelorCoin>(listing) == option::none(), 0);
        assert!(coin_listing::current_bidder<VelorCoin>(listing) == option::none(), 0);

        coin_listing::purchase<VelorCoin>(purchaser, listing);

        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10494, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9500, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_bid_then_purchase(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);

        coin_listing::bid<VelorCoin>(seller, listing, 100);
        assert!(coin_listing::current_amount<VelorCoin>(listing) == option::some(100), 0);
        assert!(coin_listing::current_bidder<VelorCoin>(listing) == option::some(seller_addr), 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 3, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9897, 0);

        // Return the bid and insert a new bid
        coin_listing::bid<VelorCoin>(purchaser, listing, 150);
        assert!(coin_listing::current_amount<VelorCoin>(listing) == option::some(150), 0);
        assert!(coin_listing::current_bidder<VelorCoin>(listing) == option::some(purchaser_addr), 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 5, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9997, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9848, 0);

        // Return the bid and replace with a purchase
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 10, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9498, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_bidding(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
        let end_time = timestamp::now_seconds() + 200;
        assert!(coin_listing::auction_end_time<VelorCoin>(listing) == end_time, 0);

        // Bid but do not affect end timing
        coin_listing::bid<VelorCoin>(seller, listing, 100);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 3, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9897, 0);
        assert!(coin_listing::auction_end_time<VelorCoin>(listing) == end_time, 0);

        // Return the bid and insert a new bid and affect end timing
        test_utils::increment_timestamp(150);
        coin_listing::bid<VelorCoin>(purchaser, listing, 150);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 5, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9997, 0);
        assert!(coin::balance<VelorCoin>(purchaser_addr) == 9848, 0);
        assert!(coin_listing::auction_end_time<VelorCoin>(listing) != end_time, 0);

        // End the auction as out of time
        test_utils::increment_timestamp(150);
        coin_listing::complete_auction<VelorCoin>(velor_framework, listing);
        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 10146, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_ended_auction_no_bid(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, _purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);

        test_utils::increment_timestamp(200);
        coin_listing::complete_auction<VelorCoin>(velor_framework, listing);

        assert!(object::owner(token) == seller_addr, 0);
        assert!(coin::balance<VelorCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<VelorCoin>(seller_addr) == 9999, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30002, location = marketplace::listing)]
    fun test_not_started_fixed_price(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds() + 1,
            500,
        );

        coin_listing::purchase<VelorCoin>(purchaser, listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30002, location = marketplace::listing)]
    fun test_not_started_auction(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds() + 1,
            100,
            50,
            timestamp::now_seconds() + 200,
            150,
            option::some(500),
        );

        coin_listing::bid<VelorCoin>(purchaser, listing, 1000);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30005, location = marketplace::coin_listing)]
    fun test_ended_auction_bid(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        test_utils::increment_timestamp(200);
        coin_listing::bid<VelorCoin>(purchaser, listing, 1000);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30005, location = marketplace::coin_listing)]
    fun test_ended_auction_purchase(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        test_utils::increment_timestamp(200);
        coin_listing::purchase<VelorCoin>(purchaser, listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10004, location = velor_framework::fungible_asset)]
    fun test_not_enough_coin_fixed_price(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            100000,
        );

        coin_listing::purchase<VelorCoin>(purchaser, listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10004, location = velor_framework::fungible_asset)]
    fun test_not_enough_coin_auction_bid(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        coin_listing::bid<VelorCoin>(purchaser, listing, 100000);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10003, location = marketplace::coin_listing)]
    fun test_bid_too_low(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        coin_listing::bid<VelorCoin>(purchaser, listing, 100);
        coin_listing::bid<VelorCoin>(purchaser, listing, 125);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10004, location = velor_framework::fungible_asset)]
    fun test_not_enough_coin_auction_purchase(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            100,
            50,
            timestamp::now_seconds() + 200,
            150,
            option::some(50000),
        );

        coin_listing::purchase<VelorCoin>(purchaser, listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x60001, location = marketplace::coin_listing)]
    fun test_auction_view_on_fixed_price(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);
        coin_listing::auction_end_time<VelorCoin>(listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10002, location = marketplace::coin_listing)]
    fun test_purchase_on_auction_without_buy_it_now(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            100,
            50,
            timestamp::now_seconds() + 200,
            150,
            option::none(),
        );

        coin_listing::purchase<VelorCoin>(purchaser, listing);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x50006, location = marketplace::coin_listing)]
    fun test_bad_fixed_price_end(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);
        coin_listing::end_fixed_price<VelorCoin>(purchaser, listing);
    }

    // Objects and TokenV2 stuff

    inline fun fixed_price_listing(
        marketplace: &signer,
        seller: &signer,
    ): (Object<Token>, Object<FeeSchedule>, Object<Listing>) {
        let token = test_utils::mint_tokenv2(seller);
        fixed_price_listing_with_token(marketplace, seller, token)
    }

    inline fun fixed_price_listing_with_token(
        marketplace: &signer,
        seller: &signer,
        token: Object<Token>
    ): (Object<Token>, Object<FeeSchedule>, Object<Listing>) {
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            500,
        );
        (token, fee_schedule, listing)
    }


    inline fun auction_listing(
        marketplace: &signer,
        seller: &signer,
    ): (Object<Token>, Object<FeeSchedule>, Object<Listing>) {
        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<VelorCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            100,
            50,
            timestamp::now_seconds() + 200,
            150,
            option::some(500),
        );
        (token, fee_schedule, listing)
    }

    // TokenV1

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_for_token_v1(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);

        let (token_id, _fee_schedule, listing) = fixed_price_listing_for_tokenv1(marketplace, seller);
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_for_token_v1_high_royalty(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);
        let _token = mint_tokenv1(seller);
        let token_id = mint_tokenv1_additional_royalty(seller, 100, 100);

        let (_fee_schedule, listing) = fixed_price_listing_for_tokenv1_with_token(marketplace, seller, &token_id);
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        // TODO balance checks
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_for_token_v1_bad_royalty(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);
        let _token = mint_tokenv1(seller);
        let token_id = mint_tokenv1_additional_royalty(seller, 0, 0);

        let (_fee_schedule, listing) = fixed_price_listing_for_tokenv1_with_token(marketplace, seller, &token_id);
        // This should not fail, and no royalty is taken
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
        // TODO balance checks
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase_for_tokenv1(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);

        let (token_id, _fee_schedule, listing) = auction_listing_for_tokenv1(marketplace, seller);
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase_for_tokenv1_without_direct_transfer(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token_id, _fee_schedule, listing) = auction_listing_for_tokenv1(marketplace, seller);
        coin_listing::purchase<VelorCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    #[test(velor_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_win_for_tokenv1_without_direct_transfer_and_non_winner_completer(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(velor_framework, marketplace, seller, purchaser);

        let (token_id, _fee_schedule, listing) = auction_listing_for_tokenv1(marketplace, seller);
        coin_listing::bid<VelorCoin>(purchaser, listing, 100);
        test_utils::increment_timestamp(1000);
        let token_object = listing::listed_object(listing);
        coin_listing::complete_auction<VelorCoin>(velor_framework, listing);
        listing::extract_tokenv1(purchaser, object::convert(token_object));
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    inline fun fixed_price_listing_for_tokenv1(
        marketplace: &signer,
        seller: &signer,
    ): (tokenv1::TokenId, Object<FeeSchedule>, Object<Listing>) {
        let token_id = test_utils::mint_tokenv1(seller);
        let (fee_schedule, listing) = fixed_price_listing_for_tokenv1_with_token(marketplace, seller, &token_id);
        (token_id, fee_schedule, listing)
    }

    inline fun fixed_price_listing_for_tokenv1_with_token(
        marketplace: &signer,
        seller: &signer,
        token_id: &tokenv1::TokenId,
    ): (Object<FeeSchedule>, Object<Listing>) {
        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(token_id);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_for_tokenv1_internal<VelorCoin>(
            seller,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            fee_schedule,
            timestamp::now_seconds(),
            500,
        );
        (fee_schedule, listing)
    }

    inline fun auction_listing_for_tokenv1(
        marketplace: &signer,
        seller: &signer,
    ): (tokenv1::TokenId, Object<FeeSchedule>, Object<Listing>) {
        let token_id = test_utils::mint_tokenv1(seller);
        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_for_tokenv1_internal<VelorCoin>(
            seller,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            fee_schedule,
            timestamp::now_seconds(),
            100,
            50,
            timestamp::now_seconds() + 200,
            150,
            option::some(500),
        );
        (token_id, fee_schedule, listing)
    }
}
}
