/// This is an example demonstrating how to use marketplace_bid_utils and market_place_listing_utils to build an auction house
/// The basic flow can be found in test test_listing_one_and_two_bids
/// For more detailed description, check readme
module marketplace::marketplace_auction_example {

    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use marketplace::marketplace_bid_utils::{Self as bid, BidId, create_bid_id};
    use marketplace::marketplace_listing_utils::{Self as listing, Listing, ListingEvent};
    use std::error;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_framework::guid;
    use aptos_token::property_map;

    //
    // Errors
    //

    /// Expiration time is invalid
    const EINVALID_EXPIRATION_TIME: u64 = 1;

    /// Start time is invalid
    const EINVALID_START_TIME: u64 = 2;

    /// Auction doesn't exist
    const EAUCTION_NOT_EXIST: u64 = 3;

    /// Bid increase less than minimal incremental
    const EBID_INCREASE_TOO_SMALL: u64 = 4;

    /// Auction ended
    const EAUCTION_ENDED: u64 = 5;

    /// Minimal incremental should be bigger than 0
    const EBID_MIN_INCREMENTAL_IS_ZERO: u64 = 6;

    /// Bid not found
    const EBID_NOT_FOUND_FOR_AUCTION: u64 = 7;

    /// Reserved operation for auction house owner
    const EONLY_AUCTION_HOUSE_OWNER_CAN_PERFORM_THIS_OPERATION: u64 = 8;

    /// Auction not ended
    const EAUCTION_NOT_ENDED: u64 = 9;

    /// Bid with same price exists for this auction
    const EBID_WITH_SAME_PRICE_EXISTS: u64 = 10;

    /// Bid not match the bid_id in the auction
    const EBID_NOT_MATCH_ID_IN_AUCTION: u64 = 11;

    /// Auction has zero bids
    const EAUCION_HAS_ZERO_BIDS: u64 = 12;

    /// Auction highest bid is zero
    const EAUCTION_HIGHEST_BID_ZERO: u64 = 13;

    struct AuctionHouseConfig has key {
        market_fee_numerator: u64,
        market_fee_denominator: u64,
        fee_address: address,
    }

    struct Auctions<phantom CoinType> has key {
        cur_auction_id: u64, // this is used to generate next auction_id
        all_active_auctions: Table<u64, Auction<CoinType>>,
        listing_event: EventHandle<ListingEvent>,
        bid_event: EventHandle<BidEvent>,
        cancel_bid_events: EventHandle<CancelBidEvent>
    }

    struct BidEvent has copy, drop, store {
        market_address: address,
        bid_id: BidId,
        offer_price: u64,
        expiration_sec: u64,
    }

    struct CancelBidEvent has copy, drop, store {
        market_address: address,
        bid_id: BidId,
    }

    struct Auction<phantom CoinType> has drop, store {
        listing: Listing<CoinType>,
        bids: SimpleMap<u64, BidId>, // mapping between the price and BidId
        offer_numbers: vector<u64>, // the prices recorded for all the bids
    }

    public entry fun initialize_auction_house(
        account: &signer,
        market_fee_numerator: u64,
        market_fee_denominator: u64,
        fee_address: address,
    ) {
        move_to(
            account,
            AuctionHouseConfig {
                market_fee_denominator,
                market_fee_numerator,
                fee_address,
            }
        );
    }

    public entry fun initialize_auction<CoinType>(account: &signer) {
        move_to(
            account,
            Auctions<CoinType> {
                cur_auction_id: 0,
                all_active_auctions: table::new(),
                listing_event: account::new_event_handle<ListingEvent>(account),
                bid_event: account::new_event_handle<BidEvent>(account),
                cancel_bid_events: account::new_event_handle<CancelBidEvent>(account),

            }
        );
    }

    public fun generate_auction_data<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        start_sec: u64, // specify when the auction starts
        expiration_sec: u64, // specify when the auction ends
        withdraw_expiration_sec: u64,
    ): Auction<CoinType> {
        let sec = timestamp::now_seconds();
        assert!(sec <= start_sec, error::invalid_argument(EINVALID_START_TIME));
        assert!(start_sec < expiration_sec, error::invalid_argument(EINVALID_EXPIRATION_TIME));
        let listing = listing::create_listing<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            false,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
        );

        Auction<CoinType>{
            listing,
            bids: simple_map::create(),
            offer_numbers: vector::empty(),
        }
    }

    public entry fun create_auction<CoinType>(
        owner: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
        amount: u64,
        min_price: u64,
        start_sec: u64, // specify when the auction starts
        expiration_sec: u64, // specify when the auction ends
        withdraw_expiration_sec: u64, // specify deadline of token withdraw
    ) acquires Auctions {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_auction_with_token_id<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
        );
    }

    public fun create_auction_with_token_id<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        start_sec: u64, // specify when the auction starts
        listing_expiration_sec: u64, // specify when the auction ends
        withdraw_expiration_sec: u64, // specify deadline of token withdraw
    ): u64 acquires Auctions {

        let auction = generate_auction_data<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            start_sec,
            listing_expiration_sec,
            withdraw_expiration_sec, // allow time to withdraw
        );

        // initialized coin store when listing
        if (!coin::is_account_registered<CoinType>(signer::address_of(owner))) {
            coin::register<CoinType>(owner);
        };

        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        event::emit_event<ListingEvent>(
            &mut auctions.listing_event,
            listing::create_listing_event(
                listing::get_listing_id(&auction.listing),
                token_id,
                amount,
                min_price,
                false,
                start_sec,
                listing_expiration_sec,
                listing_expiration_sec + 50,
                @marketplace,
                property_map::empty(),
            ),
        );

        let next_id = auctions.cur_auction_id + 1;
        *(&mut auctions.cur_auction_id) = next_id;
        table::add(&mut auctions.all_active_auctions, next_id, auction);
        next_id
    }

    public entry fun bid<CoinType>(
        bidder: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
        token_amount:u64,
        offer_price: u64,
        auction_id: u64,
        withdraw_expiration_sec: u64,
    ) acquires Auctions {
        // create bid and store it under the user account
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_bid_with_token_id<CoinType>(bidder, token_id, token_amount, offer_price, auction_id, withdraw_expiration_sec);
    }

    /// Allow the bid to increase the coin for an existing bid
    public entry fun increase_bid<CoinType>(
        bidder: &signer,
        price_delta: u64,
        auction_id: u64,
    ) acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&mut auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);
        let listing_id = listing::get_listing_id<CoinType>(&auction.listing);
        let bid_id = bid::create_bid_id(signer::address_of(bidder), listing_id);
        increase_bid_price<CoinType>(
            bidder,
            bid_id,
            price_delta,
            auction_id,
        )
    }


    /// Increase the offered price for an existing bid
    /// The new price should not be same as any existing offered price
    public fun increase_bid_price<CoinType>(
        bidder: &signer,
        bid_id: BidId,
        price_delta: u64,
        auction_id: u64,
    ) acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&mut auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);

        // get the listing info
        // auction is still active
        let now = timestamp::now_seconds();
        assert!(now <= listing::get_listing_expiration<CoinType>(&auction.listing), error::invalid_argument(EAUCTION_ENDED));


        // assert new offer_price is not duplicate price
        let (old_price, _) = bid::get_bid_info<CoinType>(bid_id);
        let new_offer_price = old_price + price_delta;
        // check if same price exists previously, only bid with a different price can enter the auction
        assert!(!simple_map::contains_key(&auction.bids, &new_offer_price), error::already_exists(EBID_WITH_SAME_PRICE_EXISTS));

        bid::increase_bid(
            bidder,
            bid_id,
            price_delta,
            &auction.listing
        );
    }

    public fun create_bid_with_token_id<CoinType>(
        bidder: &signer,
        token_id: TokenId,
        token_amount:u64,
        offer_price: u64,
        auction_id: u64,
        withdraw_expiration_sec: u64,
    ): BidId acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&mut auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);

        // initialize token store when bidding
        token::initialize_token_store(bidder);

        // get the listing info
        // auction is still active
        let now = timestamp::now_seconds();
        assert!(now <= listing::get_listing_expiration<CoinType>(&auction.listing), error::invalid_argument(EAUCTION_ENDED));

        // check if same price exists previously, only bid with a different price can enter the auction
        assert!(!simple_map::contains_key(&auction.bids, &offer_price), error::already_exists(EBID_WITH_SAME_PRICE_EXISTS));

        // allow participant to withdraw coin 60 secs after auction ends, configurable by each marketplace
        let bid_id = bid::bid<CoinType>(
            bidder,
            token_id,
            token_amount,
            offer_price * token_amount,
            &auction.listing,
            withdraw_expiration_sec,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
        );

        event::emit_event<BidEvent>(
            &mut auctions.bid_event,
            BidEvent {
                market_address: @marketplace,
                bid_id,
                offer_price,
                expiration_sec: withdraw_expiration_sec,
            },
        );

        // store the bid for this auction, only higher bid can enter the auction.
        simple_map::add(&mut auction.bids, offer_price, bid_id);
        vector::push_back(&mut auction.offer_numbers, offer_price);
        bid_id
    }

    /// Auction house owner can remove auction from inventory
    public fun remove_auction<CoinType>(account: &signer, auction_id: u64): Auction<CoinType> acquires Auctions {
        assert!(signer::address_of(account) == @marketplace, error::permission_denied(EONLY_AUCTION_HOUSE_OWNER_CAN_PERFORM_THIS_OPERATION));
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        table::remove(&mut auctions.all_active_auctions, auction_id)
    }

    /// Complete the auction, select the highest bid from existing bids and execute the bid against the listing
    public entry fun complete_auction<CoinType>(account: &signer, auction_id: u64) acquires Auctions, AuctionHouseConfig {
        assert!(signer::address_of(account) == @marketplace, error::permission_denied(EONLY_AUCTION_HOUSE_OWNER_CAN_PERFORM_THIS_OPERATION));
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));

        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);
        let expiration_time = listing::get_listing_expiration<CoinType>(&auction.listing);
        let now = timestamp::now_seconds();
        assert!(now >= expiration_time, error::invalid_state(EAUCTION_NOT_ENDED));

        let config = borrow_global<AuctionHouseConfig>(@marketplace);
        let auction = remove_auction<CoinType>(account, auction_id);
        let highest_bid_id = find_highest_bid(&auction);

        let Auction {
            listing,
            bids,
            offer_numbers: _,
        } = auction;


        if ( simple_map::length(&bids) > 0) {
            // get the bid corresponding to highest price
            bid::execute_listing_bid<CoinType>(
                highest_bid_id,
                listing,
                config.fee_address,
                config.market_fee_numerator,
                config.market_fee_denominator,
            );
        };
    }

    /// The same function exists in the marketplace bid utils.
    /// Have this function here is to make the marketplace feature complete since the marketplace contract should also
    /// allow users an entry function to withdraw coin
    public entry fun withdraw_coin_from_bid<CoinType>(
        bidder: &signer,
        lister_addr: address,
        listing_creation_number: u64,
    ) {
        bid::withdraw_coin_from_bid<CoinType>(bidder, lister_addr, listing_creation_number);
    }

    /// bidder can remove their bid from the auction so that the bid won't participate in auction
    /// This doesn't withdraw the actual coin from the bid
    public entry fun cancel_bid_in_auction<CoinType>(
        bidder: &signer,
        auction_id: u64,
    ) acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);

        let now = timestamp::now_seconds();
        assert!(now < listing::get_listing_expiration<CoinType>(&auction.listing), error::invalid_argument(EAUCTION_ENDED));

        let listing_id = listing::get_listing_id<CoinType>(&auction.listing);
        let bidder_address = signer::address_of(bidder);
        let bid_id = create_bid_id(bidder_address, listing_id);

        let (offer_price, _) = bid::get_bid_info<CoinType>(bid_id);
        assert!(simple_map::contains_key(&mut auction.bids, &offer_price), error::not_found(EBID_NOT_FOUND_FOR_AUCTION));
        assert!(
            *simple_map::borrow(&mut auction.bids, &offer_price) == bid_id,
            error::permission_denied(EBID_NOT_MATCH_ID_IN_AUCTION)
        );

        simple_map::remove(&mut auction.bids, &offer_price);
        let (found, index) = vector::index_of(&mut auction.offer_numbers, &offer_price);
        assert!(found, error::not_found(EBID_NOT_FOUND_FOR_AUCTION));
        vector::swap_remove(&mut auction.offer_numbers, index);

        event::emit_event<CancelBidEvent>(
            &mut auctions.cancel_bid_events,
            CancelBidEvent {
                market_address: @marketplace,
                bid_id,
            },
        );
    }

    /// Get the listing id corresponding to a auction
    public fun get_auction_listing_id<CoinType>(auction_id: u64): guid::ID acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@marketplace);
        assert!(table::contains(&auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);

       listing::get_listing_id<CoinType>(&auction.listing)
    }

    fun find_highest_bid<CoinType>(auction: &Auction<CoinType>): BidId {
        assert!(simple_map::length(&auction.bids) > 0, error::invalid_state(EAUCION_HAS_ZERO_BIDS));
        let highest_price = 0;
        let ind = 0;
        while (ind < vector::length(&auction.offer_numbers)) {
            let price = *vector::borrow(&auction.offer_numbers, ind);
            if (price > highest_price) {
                highest_price = price;
            };
            ind = ind + 1;
        };
        assert!(highest_price > 0, error::invalid_state(EAUCTION_HIGHEST_BID_ZERO));
        *simple_map::borrow(&auction.bids, &highest_price)
    }

    #[test(lister = @0xAF, bidder_a = @0xBB, bidder_b = @0xBA, framework = @0x1, house = @marketplace, fee_account = @0xa)]
    public fun test_listing_one_and_two_bids(
        lister: signer,
        bidder_a: signer,
        bidder_b: signer,
        framework: signer,
        house: signer,
        fee_account: signer,
    ) acquires Auctions, AuctionHouseConfig {
        use aptos_framework::coin;
        use aptos_framework::account;
        timestamp::set_time_has_started_for_testing(&framework);
        timestamp::update_global_time_for_test(1);
        account::create_account_for_test(signer::address_of(&lister));
        account::create_account_for_test(signer::address_of(&bidder_a));
        account::create_account_for_test(signer::address_of(&bidder_b));
        account::create_account_for_test(signer::address_of(&framework));
        account::create_account_for_test(signer::address_of(&fee_account));


        // setup the auction house global fee config and config for each coin type
        initialize_auction_house(
            &house,
            1,
            100,
            signer::address_of(&fee_account)
        );
        initialize_auction<coin::FakeMoney>(&house);

        // owner creats a listing
        let token_id = token::create_collection_and_token(
            &lister,
            2,
            2,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );
        let (creator, collection, name, version) = token::get_token_id_fields(&token_id);
        create_auction<coin::FakeMoney>(
            &lister,
            creator,
            collection,
            name,
            version,
            1,
            1,
            1,
            2,
            2 + 10,
        );

        timestamp::update_global_time_for_test(1000000);

        coin::create_fake_money(&framework, &bidder_a, 1000);
        coin::register<coin::FakeMoney>(&bidder_b);
        coin::register<coin::FakeMoney>(&fee_account);
        coin::transfer<coin::FakeMoney>(&framework, signer::address_of(&bidder_a), 500);
        coin::transfer<coin::FakeMoney>(&framework, signer::address_of(&bidder_b), 500);

        bid<coin::FakeMoney>(
            &bidder_a,
            creator,
            collection,
            name,
            version,
            1,
            100,
            1,
            1 + 10,
        );

        bid<coin::FakeMoney>(
            &bidder_b,
            creator,
            collection,
            name,
            version,
            1,
            300,
            1,
            1 + 10,
        );

        timestamp::update_global_time_for_test(3000000);

        complete_auction<coin::FakeMoney>(&house, 1);

        // highest bidder bidder B get the token
        assert!(token::balance_of(signer::address_of(&bidder_b), token_id) == 1, 1);
        assert!(token::balance_of(signer::address_of(&bidder_a), token_id) == 0, 1);
        // 3 coin is paid for market fee and remaining is 297
        assert!(coin::balance<coin::FakeMoney>(signer::address_of(&lister)) == 297, 1);
    }

    #[test(lister = @0xAF, bidder_a = @0xBB, framework = @0x1, house = @marketplace)]
    public fun test_cancel_bid(
        lister: signer,
        bidder_a: signer,
        framework: signer,
        house: signer,
    ) acquires Auctions {
        use aptos_framework::coin;
        use aptos_framework::account;
        timestamp::set_time_has_started_for_testing(&framework);
        timestamp::update_global_time_for_test(1);
        account::create_account_for_test(signer::address_of(&lister));
        account::create_account_for_test(signer::address_of(&bidder_a));
        account::create_account_for_test(signer::address_of(&framework));

        // setup the auction house global fee config and config for each coin type
        initialize_auction_house(
            &house,
            1,
            100,
            signer::address_of(&house)
        );
        initialize_auction<coin::FakeMoney>(&house);

        // owner creats a listing
        let token_id = token::create_collection_and_token(
            &lister,
            2,
            2,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );

        let auction_id = create_auction_with_token_id<coin::FakeMoney>(
            &lister,
            token_id,
            1,
            1,
            12,
            20,
            20 + 50,
        );
        coin::create_fake_money(&framework, &bidder_a, 1000);
        coin::transfer<coin::FakeMoney>(&framework, signer::address_of(&bidder_a), 500);

        timestamp::update_global_time_for_test(12000000);

        create_bid_with_token_id<coin::FakeMoney>(
            &bidder_a,
            token_id,
            1,
            100,
            auction_id,
            20 + 50,
        );

        // bid_id_creation_number should be shown to users to allow them cancel the bid
        cancel_bid_in_auction<coin::FakeMoney>(&bidder_a, auction_id);
        let auction = table::borrow(
            &borrow_global<Auctions<coin::FakeMoney>>(signer::address_of(&house)).all_active_auctions,
            auction_id
        );
        assert!(simple_map::length(&auction.bids) == 0, 1);
        timestamp::update_global_time_for_test(300000000);

        let listing_id = get_auction_listing_id<coin::FakeMoney>(auction_id);
        withdraw_coin_from_bid<coin::FakeMoney>(
            &bidder_a,
            guid::id_creator_address(&listing_id),
            guid::id_creation_num(&listing_id)
        );
    }
}
