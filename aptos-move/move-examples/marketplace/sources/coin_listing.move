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
    use std::string::String;

    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::object::{Self, ConstructorRef, Object, ObjectCore};
    use aptos_framework::timestamp;

    use marketplace::fee_schedule::{Self, FeeSchedule};
    use marketplace::listing::{Self, Listing};

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

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Fixed-price market place listing.
    struct FixedPriceListing<phantom CoinType> has key {
        /// The price to purchase the item up for listing.
        price: u64,
        /// Purchase event -- as it is only ever executed once.
        purchase_event: EventHandle<PurchaseEvent>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
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
        /// Bid events
        bid_events: EventHandle<BidEvent>,
        /// Purchase event -- as it is only ever executed once.
        purchase_event: EventHandle<PurchaseEvent>,
    }

    /// Represents a single bid within this auction house.
    struct Bid<phantom CoinType> has store {
        bidder: address,
        coins: Coin<CoinType>,
    }

    /// An event triggered upon each bid.
    struct BidEvent has drop, store {
        new_bidder: address,
        new_bid: u64,
        new_end_time: u64,
        previous_bidder: Option<address>,
        previous_bid: Option<u64>,
        previous_end_time: u64,
    }

    /// An event triggered upon the sale of an item. Note, the amount given to the seller is the
    /// price - commission - royalties. In the case there was no sale, purchaser is equal to seller
    /// and the amounts will all be zero.
    struct PurchaseEvent has drop, store {
        purchaser: address,
        price: u64,
        commission: u64,
        royalties: u64,
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
            purchase_event: object::new_event_handle(&listing_signer),
        };
        move_to(&listing_signer, fixed_price_listing);

        object::object_from_constructor_ref(&constructor_ref)
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
            bid_events: object::new_event_handle(&listing_signer),
            purchase_event: object::new_event_handle(&listing_signer),
        };
        move_to(&listing_signer, auction_listing);
        object::object_from_constructor_ref(&constructor_ref)
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
        coin::transfer<CoinType>(
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
        let (price, purchase_event) = if (exists<AuctionListing<CoinType>>(listing_addr)) {
            let AuctionListing {
                starting_bid: _,
                bid_increment: _,
                current_bid,
                auction_end_time,
                minimum_bid_time_before_end: _,
                buy_it_now_price,
                bid_events,
                purchase_event,
            } = move_from<AuctionListing<CoinType>>(listing_addr);

            let now = timestamp::now_seconds();
            assert!(now < auction_end_time, error::invalid_state(EAUCTION_ENDED));

            assert!(option::is_some(&buy_it_now_price), error::invalid_argument(ENO_BUY_IT_NOW));
            if (option::is_some(&current_bid)) {
                let Bid { bidder, coins } = option::destroy_some(current_bid);
                coin::deposit(bidder, coins);
            } else {
                option::destroy_none(current_bid);
            };
            event::destroy_handle(bid_events);
            (option::destroy_some(buy_it_now_price), purchase_event)
        } else if (exists<FixedPriceListing<CoinType>>(listing_addr)) {
            let FixedPriceListing {
                price,
                purchase_event,
            } = move_from<FixedPriceListing<CoinType>>(listing_addr);
            (price, purchase_event)
        } else {
            // This should just be an abort but the compiler errors.
            abort(error::not_found(ENO_LISTING))
        };

        let coins = coin::withdraw<CoinType>(purchaser, price);

        complete_purchase(signer::address_of(purchaser), object, purchase_event, coins)
    }

    /// End a fixed price listing early.
    public entry fun end_fixed_price<CoinType>(
        seller: &signer,
        object: Object<Listing>,
    ) acquires FixedPriceListing {
        let expected_seller_addr = signer::address_of(seller);
        let (actual_seller_addr, _fee_schedule) = listing::close(object, expected_seller_addr);
        assert!(expected_seller_addr == actual_seller_addr, error::permission_denied(ENOT_SELLER));

        let listing_addr = object::object_address(&object);
        assert!(exists<FixedPriceListing<CoinType>>(listing_addr), error::not_found(ENO_LISTING));
        let FixedPriceListing {
            price: _,
            purchase_event,
        } = move_from<FixedPriceListing<CoinType>>(listing_addr);

        let purchase_event_data = PurchaseEvent {
            purchaser: expected_seller_addr,
            price: 0,
            commission: 0,
            royalties: 0,
        };
        event::emit_event(&mut purchase_event, purchase_event_data);
        event::destroy_handle(purchase_event);
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
            coin::deposit(bidder, coins);
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
        coin::transfer<CoinType>(
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

        let bid_event_data = BidEvent {
            new_bidder: signer::address_of(bidder),
            new_bid: bid_amount,
            new_end_time: auction_listing.auction_end_time,
            previous_bidder,
            previous_bid,
            previous_end_time: current_end_time,
        };
        event::emit_event(&mut auction_listing.bid_events, bid_event_data);
    }

    /// Once the current time has elapsed the auctions run time, allow the auction to be settled by
    /// distributing out the asset to the winner or the auction seller if no one bid as well as
    /// giving any fees to the marketplace that hosted the auction.
    public entry fun complete_auction<CoinType>(
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
            bid_events,
            purchase_event,
        } = move_from<AuctionListing<CoinType>>(listing_addr);

        let now = timestamp::now_seconds();
        assert!(auction_end_time <= now, error::invalid_state(EAUCTION_NOT_ENDED));

        let seller = listing::seller(object);

        let (purchaser, coins)  = if (option::is_some(&current_bid)) {
            let Bid { bidder, coins } = option::destroy_some(current_bid);
            (bidder, coins)
        } else {
            option::destroy_none(current_bid);
            (seller, coin::zero<CoinType>())
        };

        complete_purchase(purchaser, object, purchase_event, coins);
        event::destroy_handle(bid_events);
    }

    inline fun complete_purchase<CoinType>(
        purchaser_addr: address,
        object: Object<Listing>,
        purchase_event: EventHandle<PurchaseEvent>,
        coins: Coin<CoinType>,
    ) {
        let price = coin::value(&coins);
        let (royalty_addr, royalty_charge) = listing::compute_royalty(object, price);
        let (seller, fee_schedule) = listing::close(object, purchaser_addr);

        let commission_charge = fee_schedule::commission(fee_schedule, price);
        let commission = coin::extract(&mut coins, commission_charge);
        coin::deposit(fee_schedule::fee_address(fee_schedule), commission);

        if (royalty_charge != 0) {
            let royalty = coin::extract(&mut coins, royalty_charge);
            coin::deposit(royalty_addr, royalty);
        };

        coin::deposit(seller, coins);

        let purchase_event_data = PurchaseEvent {
            purchaser: purchaser_addr,
            price,
            commission: commission_charge,
            royalties: royalty_charge,
        };
        event::emit_event(&mut purchase_event, purchase_event_data);
        event::destroy_handle(purchase_event);
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

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::timestamp;

    use aptos_token::token as tokenv1;

    use aptos_token_objects::token::Token;

    use marketplace::coin_listing;
    use marketplace::fee_schedule::FeeSchedule;
    use marketplace::listing::{Self, Listing};
    use marketplace::test_utils;

    fun test_fixed_price(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, fee_schedule, listing) = fixed_price_listing(marketplace, seller);

        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);
        assert!(listing::listed_object(listing) == object::convert(token), 0);
        assert!(listing::fee_schedule(listing) == fee_schedule, 0);
        assert!(coin_listing::price<AptosCoin>(listing) == option::some(500), 0);
        assert!(!coin_listing::is_auction<AptosCoin>(listing), 0);

        coin_listing::purchase<AptosCoin>(purchaser, listing);

        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 10494, 0);
        assert!(coin::balance<AptosCoin>(purchaser_addr) == 9500, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_end(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, _purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);

        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        coin_listing::end_fixed_price<AptosCoin>(seller, listing);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);
        assert!(object::owner(token) == seller_addr, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);
        assert!(listing::listed_object(listing) == object::convert(token), 0);
        assert!(listing::fee_schedule(listing) == fee_schedule, 0);
        assert!(coin_listing::price<AptosCoin>(listing) == option::some(500), 0);
        assert!(coin_listing::is_auction<AptosCoin>(listing), 0);
        assert!(coin_listing::starting_bid<AptosCoin>(listing) == 100, 0);
        assert!(coin_listing::bid_increment<AptosCoin>(listing) == 50, 0);
        assert!(coin_listing::auction_end_time<AptosCoin>(listing) == timestamp::now_seconds() + 200, 0);
        assert!(coin_listing::minimum_bid_time_before_end<AptosCoin>(listing) == 150, 0);
        assert!(coin_listing::current_amount<AptosCoin>(listing) == option::none(), 0);
        assert!(coin_listing::current_bidder<AptosCoin>(listing) == option::none(), 0);

        coin_listing::purchase<AptosCoin>(purchaser, listing);

        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 10494, 0);
        assert!(coin::balance<AptosCoin>(purchaser_addr) == 9500, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_bid_then_purchase(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);

        coin_listing::bid<AptosCoin>(seller, listing, 100);
        assert!(coin_listing::current_amount<AptosCoin>(listing) == option::some(100), 0);
        assert!(coin_listing::current_bidder<AptosCoin>(listing) == option::some(seller_addr), 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 3, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9897, 0);

        // Return the bid and insert a new bid
        coin_listing::bid<AptosCoin>(purchaser, listing, 150);
        assert!(coin_listing::current_amount<AptosCoin>(listing) == option::some(150), 0);
        assert!(coin_listing::current_bidder<AptosCoin>(listing) == option::some(purchaser_addr), 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 5, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9997, 0);
        assert!(coin::balance<AptosCoin>(purchaser_addr) == 9848, 0);

        // Return the bid and replace with a purchase
        coin_listing::purchase<AptosCoin>(purchaser, listing);
        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 10, 0);
        assert!(coin::balance<AptosCoin>(purchaser_addr) == 9498, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_bidding(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);
        let end_time = timestamp::now_seconds() + 200;
        assert!(coin_listing::auction_end_time<AptosCoin>(listing) == end_time, 0);

        // Bid but do not affect end timing
        coin_listing::bid<AptosCoin>(seller, listing, 100);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 3, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9897, 0);
        assert!(coin_listing::auction_end_time<AptosCoin>(listing) == end_time, 0);

        // Return the bid and insert a new bid and affect end timing
        test_utils::increment_timestamp(150);
        coin_listing::bid<AptosCoin>(purchaser, listing, 150);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 5, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9997, 0);
        assert!(coin::balance<AptosCoin>(purchaser_addr) == 9848, 0);
        assert!(coin_listing::auction_end_time<AptosCoin>(listing) != end_time, 0);

        // End the auction as out of time
        test_utils::increment_timestamp(150);
        coin_listing::complete_auction<AptosCoin>(listing);
        assert!(object::owner(token) == purchaser_addr, 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 6, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 10146, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_ended_auction_no_bid(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (marketplace_addr, seller_addr, _purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);

        test_utils::increment_timestamp(200);
        coin_listing::complete_auction<AptosCoin>(listing);

        assert!(object::owner(token) == seller_addr, 0);
        assert!(coin::balance<AptosCoin>(marketplace_addr) == 1, 0);
        assert!(coin::balance<AptosCoin>(seller_addr) == 9999, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30002, location = marketplace::listing)]
    fun test_not_started_fixed_price(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<AptosCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds() + 1,
            500,
        );

        coin_listing::purchase<AptosCoin>(purchaser, listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30002, location = marketplace::listing)]
    fun test_not_started_auction(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<AptosCoin>(
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

        coin_listing::bid<AptosCoin>(purchaser, listing, 1000);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30005, location = marketplace::coin_listing)]
    fun test_ended_auction_bid(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        test_utils::increment_timestamp(200);
        coin_listing::bid<AptosCoin>(purchaser, listing, 1000);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x30005, location = marketplace::coin_listing)]
    fun test_ended_auction_purchase(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        test_utils::increment_timestamp(200);
        coin_listing::purchase<AptosCoin>(purchaser, listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10006, location = aptos_framework::coin)]
    fun test_not_enough_coin_fixed_price(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
         test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<AptosCoin>(
            seller,
            object::convert(token),
            fee_schedule,
            timestamp::now_seconds(),
            100000,
        );

        coin_listing::purchase<AptosCoin>(purchaser, listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10006, location = aptos_framework::coin)]
    fun test_not_enough_coin_auction_bid(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        coin_listing::bid<AptosCoin>(purchaser, listing, 100000);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10003, location = marketplace::coin_listing)]
    fun test_bid_too_low(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = auction_listing(marketplace, seller);
        coin_listing::bid<AptosCoin>(purchaser, listing, 100);
        coin_listing::bid<AptosCoin>(purchaser, listing, 125);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10006, location = aptos_framework::coin)]
    fun test_not_enough_coin_auction_purchase(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<AptosCoin>(
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

        coin_listing::purchase<AptosCoin>(purchaser, listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x60001, location = marketplace::coin_listing)]
    fun test_auction_view_on_fixed_price(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);
        coin_listing::auction_end_time<AptosCoin>(listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x10002, location = marketplace::coin_listing)]
    fun test_purchase_on_auction_without_buy_it_now(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_internal<AptosCoin>(
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

        coin_listing::purchase<AptosCoin>(purchaser, listing);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    #[expected_failure(abort_code = 0x50006, location = marketplace::coin_listing)]
    fun test_bad_fixed_price_end(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (_token, _fee_schedule, listing) = fixed_price_listing(marketplace, seller);
        coin_listing::end_fixed_price<AptosCoin>(purchaser, listing);
    }

    // Objects and TokenV2 stuff

    inline fun fixed_price_listing(
        marketplace: &signer,
        seller: &signer,
    ): (Object<Token>, Object<FeeSchedule>, Object<Listing>) {
        let token = test_utils::mint_tokenv2(seller);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_internal<AptosCoin>(
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
        let listing = coin_listing::init_auction_internal<AptosCoin>(
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

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_fixed_price_for_token_v1(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);

        let (token_id, _fee_schedule, listing) = fixed_price_listing_for_tokenv1(marketplace, seller);
        coin_listing::purchase<AptosCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase_for_tokenv1(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);
        tokenv1::opt_in_direct_transfer(purchaser, true);

        let (token_id, _fee_schedule, listing) = auction_listing_for_tokenv1(marketplace, seller);
        coin_listing::purchase<AptosCoin>(purchaser, listing);
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    #[test(aptos_framework = @0x1, marketplace = @0x111, seller = @0x222, purchaser = @0x333)]
    fun test_auction_purchase_for_tokenv1_without_direct_transfer(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ) {
        let (_marketplace_addr, _seller_addr, purchaser_addr) =
            test_utils::setup(aptos_framework, marketplace, seller, purchaser);

        let (token_id, _fee_schedule, listing) = auction_listing_for_tokenv1(marketplace, seller);
        let token_object = listing::listed_object(listing);
        coin_listing::purchase<AptosCoin>(purchaser, listing);
        listing::extract_tokenv1(purchaser, object::convert(token_object));
        assert!(tokenv1::balance_of(purchaser_addr, token_id) == 1, 0);
    }

    inline fun fixed_price_listing_for_tokenv1(
        marketplace: &signer,
        seller: &signer,
    ): (tokenv1::TokenId, Object<FeeSchedule>, Object<Listing>) {
        let token_id = test_utils::mint_tokenv1(seller);
        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_fixed_price_for_tokenv1_internal<AptosCoin>(
            seller,
            creator_addr,
            collection_name,
            token_name,
            property_version,
            fee_schedule,
            timestamp::now_seconds(),
            500,
        );
        (token_id, fee_schedule, listing)
    }

    inline fun auction_listing_for_tokenv1(
        marketplace: &signer,
        seller: &signer,
    ): (tokenv1::TokenId, Object<FeeSchedule>, Object<Listing>) {
        let token_id = test_utils::mint_tokenv1(seller);
        let (creator_addr, collection_name, token_name, property_version) =
            tokenv1::get_token_id_fields(&token_id);
        let fee_schedule = test_utils::fee_schedule(marketplace);
        let listing = coin_listing::init_auction_for_tokenv1_internal<AptosCoin>(
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
