// support first price auction (english auction), the highest bid win the auction
// this example shows an auction house that stores user listing under auction house's account.

module auction_house::auction_house_own_listing {

    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::guid::{Self, ID};
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use auction_house::bid;
    use auction_house::listing::{Self, Listing, ListingEvent};
    use std::error;
    use std::signer;
    use std::string::String;
    use std::vector;

    //
    // Constants
    //

    const AUCTION_DELAY_TIME_SEC: u64 = 300;

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
        bid_id: ID,
    }

    struct CancelBidEvent has copy, drop, store {
        market_address: address,
        bid_id: ID,
    }

    struct Auction<phantom CoinType> has drop, store {
        listing: Listing<CoinType>,
        bids: vector<ID>,
        highest_price: u64,
        highest_bid: ID,
        min_incremental_price: u64,
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
        min_incremental_price: u64,
    ): Auction<CoinType> {
        let sec = timestamp::now_seconds();
        assert!(sec <= start_sec, error::invalid_argument(EINVALID_START_TIME));
        assert!(start_sec < expiration_sec, error::invalid_argument(EINVALID_EXPIRATION_TIME));
        let listing = listing::create_list<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            false,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
        );

        Auction<CoinType>{
            listing,
            bids: vector::empty(),
            highest_price: 0,
            highest_bid: guid::create_id(@auction_house, 0),
            min_incremental_price,
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
        min_incremental_price: u64,
    ) acquires Auctions {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_auction_with_token_id<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            start_sec,
            expiration_sec,
            min_incremental_price,
        );
    }

    public fun create_auction_with_token_id<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        start_sec: u64, // specify when the auction starts
        expiration_sec: u64, // specify when the auction ends
        min_incremental_price: u64,
    ): u64 acquires Auctions {
        let auction = generate_auction_data<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            start_sec,
            expiration_sec,
            expiration_sec + AUCTION_DELAY_TIME_SEC, // allow time to withdraw
            min_incremental_price,
        );

        // initialized coin store when listing
        if (!coin::is_account_registered<CoinType>(signer::address_of(owner))) {
            coin::register<CoinType>(owner);
        };

        let auctions = borrow_global_mut<Auctions<CoinType>>(@auction_house);
        event::emit_event<ListingEvent>(
            &mut auctions.listing_event,
            listing::create_listing_event(
                listing::get_listing_id(&auction.listing),
                token_id,
                amount,
                min_price,
                false,
                start_sec,
                expiration_sec,
                @auction_house,
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
    ) acquires Auctions {
        // create bid and store it under the user account
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_bid_with_token_id<CoinType>(bidder, token_id, token_amount, offer_price, auction_id);
    }

    public fun create_bid_with_token_id<CoinType>(
        bidder: &signer,
        token_id: TokenId,
        token_amount:u64,
        offer_price: u64,
        auction_id: u64,
    ): ID acquires Auctions {
        let auctions = borrow_global_mut<Auctions<CoinType>>(@auction_house);
        assert!(table::contains(&mut auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);
        // min_incremental_price should be bigger than 0, this also ensure the offer_price is bigger than 0
        assert!(auction.min_incremental_price > 0, error::invalid_argument(EBID_MIN_INCREMENTAL_IS_ZERO));

        // bid increase has to be bigger than smallest incremental
        assert!(
            offer_price >= auction.highest_price + auction.min_incremental_price,
            error::invalid_argument(EBID_INCREASE_TOO_SMALL)
        );

        // initialize token store when bidding
        token::initialize_token_store(bidder);

        // get the listing info
        // auction is still active
        let now = timestamp::now_seconds();
        assert!(now <= listing::get_listing_expiration<CoinType>(&auction.listing), error::invalid_argument(EAUCTION_ENDED));
        // allow participant to withdraw coin 60 secs after auction ends, configurable by each marketplace
        let bid_id = bid::bid<CoinType>(
            bidder,
            token_id,
            token_amount,
            offer_price * token_amount,
            &auction.listing,
            listing::get_listing_expiration<CoinType>(&auction.listing) + AUCTION_DELAY_TIME_SEC
        );

        event::emit_event<BidEvent>(
            &mut auctions.bid_event,
            BidEvent {
                market_address: @auction_house,
                bid_id,
            },
        );

        // store the bid for this auction, only higher bid can enter the auction.
        *(&mut auction.highest_price) = offer_price;
        *(&mut auction.highest_bid) = bid_id;
        vector::push_back(&mut auction.bids, bid_id);
        bid_id
    }

    /// bidder can cancel their bid before the auction ends to not participate the auction
    public entry fun cancel_bid<CoinType>(
        bidder: &signer,
        auction_id: u64,
        bid_id_creation_number: u64,
    ) acquires Auctions {

        let auctions = borrow_global_mut<Auctions<CoinType>>(@auction_house);
        assert!(table::contains(&auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));
        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);

        let now = timestamp::now_seconds();
        assert!(now < listing::get_listing_expiration<CoinType>(&auction.listing), error::invalid_argument(EAUCTION_ENDED));

        let bidder_address = signer::address_of(bidder);
        let bid_id = guid::create_id(bidder_address, bid_id_creation_number);
        let (offer_price, _, _) = bid::get_bid_info<CoinType>(bidder_address, bid_id_creation_number);
        //  if the bid is highest bid. find next highest bid. Otherwise, direct remove bid from auction
        let (found, ind) = vector::index_of(&mut auction.bids, &bid_id);
        assert!(found, error::not_found(EBID_NOT_FOUND_FOR_AUCTION));
        vector::remove(&mut auction.bids, ind);
        if (offer_price == auction.highest_price) {
            let new_highest_price = 0;
            let highest_bid = &guid::create_id(@auction_house, 0); // dummy bid_id
            let i = 0;
            while ( i < vector::length(&auction.bids)){
                let bid = vector::borrow(&auction.bids, i);
                let (price, _, _) = bid::get_bid_info<CoinType>(guid::id_creator_address(bid), guid::id_creation_num(bid));
                if (price > new_highest_price) {
                    new_highest_price = price;
                    highest_bid = bid;
                };
                i = i + 1;
            };
            *(&mut auction.highest_price) = new_highest_price;
            *(&mut auction.highest_bid) = *highest_bid;
        };

        event::emit_event<CancelBidEvent>(
            &mut auctions.cancel_bid_events,
            CancelBidEvent {
                market_address: @auction_house,
                bid_id,
            },
        );
    }

    /// bidder can release their coin from the bid after auction ends.
    /// the coin will be deposit back to bidder's account
    public entry fun release_coin_from_bid<CoinType>(
        bidder: &signer,
        bid_id_creation_number: u64,
    ) {
        bid::release_coin_from_bid<CoinType>(bidder, bid_id_creation_number);
    }

    /// auction house owner can remove auction from inventory
    public fun remove_auction<CoinType>(account: &signer, auction_id: u64): Auction<CoinType> acquires Auctions {
        assert!(signer::address_of(account) == @auction_house, error::permission_denied(EONLY_AUCTION_HOUSE_OWNER_CAN_PERFORM_THIS_OPERATION));
        let auctions = borrow_global_mut<Auctions<CoinType>>(@auction_house);
        table::remove(&mut auctions.all_active_auctions, auction_id)
    }

    /// complete the auction
    public entry fun complete_auction<CoinType>(account: &signer, auction_id: u64) acquires Auctions, AuctionHouseConfig {
        assert!(signer::address_of(account) == @auction_house, error::permission_denied(EONLY_AUCTION_HOUSE_OWNER_CAN_PERFORM_THIS_OPERATION));
        let auctions = borrow_global_mut<Auctions<CoinType>>(@auction_house);
        assert!(table::contains(&auctions.all_active_auctions, auction_id), error::not_found(EAUCTION_NOT_EXIST));

        let auction = table::borrow_mut(&mut auctions.all_active_auctions, auction_id);
        let expiration_time = listing::get_listing_expiration<CoinType>(&auction.listing);
        let now = timestamp::now_seconds();
        assert!(now >= expiration_time, error::invalid_state(EAUCTION_NOT_ENDED));

        let config = borrow_global<AuctionHouseConfig>(@auction_house);
        let auction = remove_auction<CoinType>(account, auction_id);

        let Auction {
            listing,
            bids,
            highest_price: _,
            highest_bid,
            min_incremental_price: _,
        } = auction;

        if ( vector::length(&bids ) > 0) {
            // get the bid corresponding to highest price
            bid::execute_listing_bid(
                highest_bid,
                listing,
                config.fee_address,
                config.market_fee_numerator,
                config.market_fee_denominator,
            );
        };
    }

    #[test(lister = @0xAF, bidder_a = @0xBB, bidder_b = @0xBA, framework = @0x1, house = @auction_house, fee_account = @0xa)]
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
        let token_id = token::create_collection_and_token(&lister, 2, 2, 2);
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
            1,
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
        );

        timestamp::update_global_time_for_test(3000000);

        complete_auction<coin::FakeMoney>(&house, 1);

        // highest bidder bidder B get the token
        assert!(token::balance_of(signer::address_of(&bidder_b), token_id) == 1, 1);
        assert!(token::balance_of(signer::address_of(&bidder_a), token_id) == 0, 1);
        // 3 coin is paid for market fee and remaining is 297
        assert!(coin::balance<coin::FakeMoney>(signer::address_of(&lister)) == 297, 1);
    }

    #[test(lister = @0xAF, bidder_a = @0xBB, framework = @0x1, house = @auction_house)]
    public fun test_cancel_bid(
        lister: signer,
        bidder_a: signer,
        framework: signer,
        house: signer,
    ) acquires Auctions, AuctionHouseConfig {
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
        let token_id = token::create_collection_and_token(&lister, 2, 2, 2);
        let auction_id = create_auction_with_token_id<coin::FakeMoney>(
            &lister,
            token_id,
            1,
            1,
            12,
            20,
            1,
        );
        coin::create_fake_money(&framework, &bidder_a, 1000);
        coin::transfer<coin::FakeMoney>(&framework, signer::address_of(&bidder_a), 500);

        timestamp::update_global_time_for_test(12000000);

        let bid_id = create_bid_with_token_id<coin::FakeMoney>(
            &bidder_a,
            token_id,
            1,
            100,
            auction_id,
        );

        // bid_id_creation_number should be shown to users to allow them cancel the bid
        cancel_bid<coin::FakeMoney>(&bidder_a, auction_id, guid::id_creation_num(&bid_id));
        let auction = table::borrow(
            &borrow_global<Auctions<coin::FakeMoney>>(signer::address_of(&house)).all_active_auctions,
            auction_id
        );
        assert!(vector::length(&auction.bids) == 0, 1);

        timestamp::update_global_time_for_test(300000000);
        complete_auction<coin::FakeMoney>(&house, auction_id);
    }
}

