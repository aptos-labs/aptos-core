// This module provides a basic Auction house with first-price auction

module AptosFramework::AuctionHouse {
    use AptosFramework::Coin::{Self, Coin};
    use AptosFramework::IterableTable::{Self, IterableTable};
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::TestCoin::TestCoin;
    use AptosFramework::Timestamp;
    use AptosFramework::Token::{Self, Token, TokenId};
    use Std::Event::{Self, EventHandle};
    use Std::GUID::{Self, ID};
    use Std::Option::{Self, Option};
    use Std::Signer;
    use Std::Vector;

    // Auction has 4 states:
    // 1/ Active: current_time >= start_time and current_time <= start_time + duration
    // 2/ Inactive: current_time < start_time or current_time > start_time + duration
    // 3/ Completed: current_time > start_time + duration
    // 4/ Unclaimed: Completed and (token not claimed or coins not claimed)

    const EINVALID_BUYER: u64 = 0;
    const EINSUFFICIENT_BID: u64 = 1;
    const ELISTING_INACTIVE: u64 = 2;
    const EAUCTION_NOT_COMPLETE: u64 = 3;
    const ENOT_CLAIMABLE: u64 = 4;
    const ETOKEN_INVOLVED_IN_ANOTHER_AUCTION: u64 = 5;
    const EALREADY_CLAIMED: u64 = 6;
    const EINVALID_TOKEN_ID: u64 = 7;
    const EINSUFFICIENT_MIN_SELLING_PRICE: u64 = 8;
    const EBUY_NOW_UNAVAILABLE: u64 = 9;

    // Represents an auction
    struct ListedItem has store {
        min_selling_price: u64,
        buy_now_price: Option<u64>, // if value not set: normal auction, elif value == min_selling_price: fixed price sale, else auction with buy it now
        duration: u64,
        start_time: u64,
        current_bid: Option<u64>,
        current_bidder: Option<address>,
        locked_tokens: Option<IterableTable<TokenId, Token>>
    }

    // Represents an escrow for holding coins until the auction finishes
    // or until someone outbids the user
    struct CoinEscrow has key {
        locked_coins: Table<ID, Coin<TestCoin>>,
    }

    // Set of data sent to the event stream when an auction is started
    struct AuctionInitEvent has store, drop {
        id: ID,
        min_selling_price: u64,
        duration: u64
    }

    // Set of data sent to the event stream when a bid is placed
    struct BidEvent has store, drop {
        id: ID,
        bid: u64,
    }

    // Set of data sent to the event stream when the seller claims the coins from the winning bidder
    struct ClaimCoinsEvent has store, drop {
        id: ID,
    }

    // Set of data sent to the event stream when the winner bidder claims the token from the seller
    struct ClaimTokenEvent has store, drop {
        id: ID,
    }

    // Set of data sent to the event stream during a listing of a token for Fixed Price Sale (FPS)
    struct FPSInitEvent has drop, store {
        id: ID,
        amount: u64,
    }

    // Set of data sent to the event stream during a buying of a token
    // this occurs at two conditions:
    // 1. Fixed Price Sale
    // 2. Auction with a buy_now_price
    struct BuyNowEvent has drop, store {
        id: ID,
        amount: u64,
    }

    // Represents data for all active or unclaimed auctions for a user
    struct ListingData has key {
        listed_items: Table<ID, ListedItem>,
        auction_init_events: EventHandle<AuctionInitEvent>,
        bid_events: EventHandle<BidEvent>,
        claim_coins_events: EventHandle<ClaimCoinsEvent>,
        claim_token_events: EventHandle<ClaimTokenEvent>,
        fps_init_events: EventHandle<FPSInitEvent>,
        buy_now_events: EventHandle<BuyNowEvent>,
    }

    fun parse_token_ids(
        sender: &signer,
        token_creators: vector<address>,
        token_collection_names: vector<vector<u8>>,
        token_names: vector<vector<u8>>
    ): IterableTable<TokenId, Token> {
        let locked_tokens = IterableTable::new<TokenId, Token>();
        let sender_addr = Signer::address_of(sender);

        let i=0;
        let n = Vector::length(&token_creators);
        while (i < n) {
            // token_id is a vector of strings in order:
            // 1. creator address
            // 2. collection name
            // 3. token name
            let token_id = Token::create_token_id_raw(
                *Vector::borrow(&token_creators, i),
                *Vector::borrow(&token_collection_names, i),
                *Vector::borrow(&token_names, i),
            );

            // check if the sender owns the token
            assert!(Token::balance_of(sender_addr, token_id) > 0, EINVALID_TOKEN_ID);

            let token = Token::withdraw_token(sender, token_id, 1);
            IterableTable::add(&mut locked_tokens, token_id, token);

            i = i + 1;
        };

        return locked_tokens
    }

    //list token for auction/ first price sale
    public(script) fun list_token_script(
        sender: &signer,
        token_creators: vector<address>,
        token_collection_names: vector<vector<u8>>,
        token_names: vector<vector<u8>>,
        min_selling_price: u64,
        buy_now_price: u64,
        duration: u64
    ) acquires ListingData {    
        assert!(min_selling_price != 0, EINSUFFICIENT_MIN_SELLING_PRICE);

        let sender_addr = Signer::address_of(sender);

        if (!exists<ListingData>(sender_addr)) {
            move_to(sender, ListingData {
                listed_items: Table::new<ID, ListedItem>(),
                auction_init_events: Event::new_event_handle<AuctionInitEvent>(sender),
                bid_events: Event::new_event_handle<BidEvent>(sender),
                claim_coins_events: Event::new_event_handle<ClaimCoinsEvent>(sender),
                claim_token_events: Event::new_event_handle<ClaimTokenEvent>(sender),
                fps_init_events: Event::new_event_handle<FPSInitEvent>(sender),
                buy_now_events: Event::new_event_handle<BuyNowEvent>(sender),
            });
        };

        if (buy_now_price == 0) {
            initialize_auction(sender, token_creators, token_collection_names, token_names, min_selling_price, Option::none<u64>(), duration);
        } else if (buy_now_price == min_selling_price) {
            initialize_fixed_price_sale(sender, token_creators, token_collection_names, token_names, min_selling_price, duration);
        } else {
            initialize_auction(sender, token_creators, token_collection_names, token_names, min_selling_price, Option::some<u64>(buy_now_price), duration);
        };
    }

    // initialize fixed price sale
    public fun initialize_fixed_price_sale(
        sender: &signer,
        token_creators: vector<address>,
        token_collection_names: vector<vector<u8>>,
        token_names: vector<vector<u8>>,
        selling_price: u64,
        duration: u64
    ) acquires ListingData{
        let sender_addr = Signer::address_of(sender);

        let start_time = Timestamp::now_microseconds();
        let listing_id = GUID::id(&GUID::create(sender));
        let listing_data = borrow_global_mut<ListingData>(sender_addr);
        let listed_items = &mut listing_data.listed_items;

        Event::emit_event<FPSInitEvent>(
            &mut listing_data.fps_init_events,
            FPSInitEvent { id: listing_id, amount: selling_price },
        );

        Table::add(listed_items, listing_id, ListedItem {
            min_selling_price: selling_price,
            buy_now_price: Option::some<u64>(selling_price),
            duration,
            start_time,
            current_bid: Option::none<u64>(),
            current_bidder: Option::none<address>(),
            locked_tokens: Option::some(parse_token_ids(sender, token_creators, token_collection_names, token_names)),
        })
    }

    // initializes an auction
    // auction starts with current bidder being the seller and current bid being min_selling_price-1
    fun initialize_auction(
        sender: &signer,
        token_creators: vector<address>,
        token_collection_names: vector<vector<u8>>,
        token_names: vector<vector<u8>>,
        min_selling_price: u64,
        buy_now_price: Option<u64>,
        duration: u64
    ) acquires ListingData {        
        let sender_addr = Signer::address_of(sender);
        
        let start_time = Timestamp::now_microseconds();
        let listing_id = GUID::id(&GUID::create(sender));
        let listing_data = borrow_global_mut<ListingData>(sender_addr);
        let listed_items = &mut listing_data.listed_items;

        Event::emit_event<AuctionInitEvent>(
            &mut listing_data.auction_init_events,
            AuctionInitEvent { id: listing_id, min_selling_price: min_selling_price, duration: duration },
        );

        Table::add(listed_items, listing_id, ListedItem {
            min_selling_price,
            buy_now_price,
            duration,
            start_time,
            current_bid:  Option::none<u64>(),
            current_bidder:  Option::none<address>(),
            locked_tokens: Option::some(parse_token_ids(sender, token_creators, token_collection_names, token_names)),
        })
    }

    fun is_auction_active(start_time: u64, duration: u64): bool {
        let current_time = Timestamp::now_microseconds();
        current_time <= start_time + duration && current_time >= start_time
    }

    fun is_auction_complete(start_time: u64, duration: u64): bool {
        let current_time = Timestamp::now_microseconds();
        current_time > start_time + duration
    }

    public(script) fun buy_token_script(
        sender: &signer,
        seller: address,
        guid_creation_num: u64
    ) acquires ListingData {
        let sender_addr = Signer::address_of(sender);
        assert!(sender_addr != seller, EINVALID_BUYER);

        let listing_id = GUID::create_id(seller, guid_creation_num);
        let listing_data = borrow_global<ListingData>(seller);
        let listed_items = &listing_data.listed_items;
        let listed_item = Table::borrow(listed_items, listing_id);

        assert!(is_auction_active(listed_item.start_time, listed_item.duration), ELISTING_INACTIVE);
        let buy_now_price = Option::get_with_default(&listed_item.buy_now_price, 0);
        assert!(listed_item.min_selling_price == buy_now_price, EBUY_NOW_UNAVAILABLE);

        buy_now(sender, seller, listing_id, buy_now_price);
    }

    fun buy_now(
        sender: &signer,
        seller: address,
        listing_id: ID,
        price: u64
    ) acquires ListingData {
        let listing_data = borrow_global_mut<ListingData>(seller);
        let listed_item = Table::borrow_mut(&mut listing_data.listed_items, listing_id);

        Event::emit_event<BuyNowEvent>(
            &mut listing_data.buy_now_events,
            BuyNowEvent { id: listing_id, amount: price },
        );

        let coin = Coin::withdraw<TestCoin>(sender, price);
        Coin::deposit(seller, coin);

        let locked_tokens = Option::extract(&mut listed_item.locked_tokens);
        let key = IterableTable::head_key<TokenId, Token>(&locked_tokens);
        while (Option::is_some(&key)) {
            let (val, _, next) = IterableTable::remove_iter<TokenId, Token>(&mut locked_tokens, *Option::borrow(&key));
            Token::deposit_token(sender, val);
            key = next;
        };

        IterableTable::destroy_empty(locked_tokens);

        let ListedItem {
            min_selling_price: _,
            buy_now_price: _,
            duration: _,
            start_time: _,
            current_bid: _,
            current_bidder: _,
            locked_tokens: locked_tokens,
        } = Table::remove(&mut listing_data.listed_items, listing_id);

        Option::destroy_none(locked_tokens);
    }

    // places a bid on an active auction
    // the placed bid should be more than the current highest bid on the auction
    public(script) fun bid_script(
        sender: &signer,
        seller: address,
        bid: u64,
        guid_creation_num: u64
    ) acquires CoinEscrow, ListingData {
        let sender_addr = Signer::address_of(sender);
        assert!(sender_addr != seller, EINVALID_BUYER);

        let listing_id = GUID::create_id(seller, guid_creation_num);
        let listing_data = borrow_global_mut<ListingData>(seller);
        let listed_items = &mut listing_data.listed_items;
        assert!(Table::contains(listed_items, listing_id), ELISTING_INACTIVE);
        let listed_item = Table::borrow_mut(listed_items, listing_id);
        assert!(is_auction_active(listed_item.start_time, listed_item.duration), ELISTING_INACTIVE);

        let current_bid = Option::get_with_default(&listed_item.current_bid, 0);
        if (current_bid != 0) { // bids should increase by 5% from the previous bids 
            assert!(bid * 100 > (current_bid * 105), EINSUFFICIENT_BID);
        } else { // if this the first bid, then it can start from min_selling_price
            assert!(bid > listed_item.min_selling_price-1, EINSUFFICIENT_BID);
        };

        // send back the deposit of the current bidder only if someone has bid before
        let current_bidder = Option::get_with_default(&listed_item.current_bidder, seller);
        if (current_bidder != seller) {
            let current_bidder_locked_coins = &mut borrow_global_mut<CoinEscrow>(current_bidder).locked_coins;
            let coins = Table::remove(current_bidder_locked_coins, listing_id);
            Coin::deposit<TestCoin>(current_bidder, coins);
        };

        if (Option::get_with_default(&listed_item.buy_now_price, 18446744073709551615) <= bid) { // max integer
            return buy_now(sender, seller, listing_id, bid)
        };

        Event::emit_event<BidEvent>(
            &mut listing_data.bid_events,
            BidEvent { id: listing_id, bid: bid },
        );

        if (!exists<CoinEscrow>(sender_addr)) {
            move_to(sender, CoinEscrow {
                locked_coins: Table::new<ID, Coin<TestCoin>>()
            });
        };

        let locked_coins = &mut borrow_global_mut<CoinEscrow>(sender_addr).locked_coins;
        let coins = Coin::withdraw<TestCoin>(sender, bid);
        Table::add(locked_coins, listing_id, coins);

        if (current_bidder == seller) {
            Option::fill(&mut listed_item.current_bidder, sender_addr);
            Option::fill(&mut listed_item.current_bid, bid);
        } else {
            Option::swap(&mut listed_item.current_bidder, sender_addr);
            Option::swap(&mut listed_item.current_bid, bid);
        };
    }

    // claims the token from the seller's account
    // called by the winning bidder, if no one bid on the auction then, winning bidder = seller
    // if the seller has not claimed their coins, deposit the escrowed coins to the seller's account
    // if both claims have been made, remove the auction item from the seller's auction data
    public(script) fun claim_token_script(
        sender: &signer,
        seller: address,
        guid_creation_num: u64
    ) acquires CoinEscrow, ListingData {
        let sender_addr = Signer::address_of(sender);

        let listing_id = GUID::create_id(seller, guid_creation_num);
        let listing_data = borrow_global_mut<ListingData>(seller);
        let listed_items = &mut listing_data.listed_items;

        assert!(Table::contains(listed_items, listing_id), ELISTING_INACTIVE);
        let listed_item = Table::borrow_mut(listed_items, listing_id);
        assert!(is_auction_complete(listed_item.start_time, listed_item.duration), EAUCTION_NOT_COMPLETE);

        let current_bidder = Option::get_with_default(&listed_item.current_bidder, seller);
        assert!(sender_addr == current_bidder, ENOT_CLAIMABLE);

        Event::emit_event<ClaimTokenEvent>(
            &mut listing_data.claim_token_events,
            ClaimTokenEvent { id: listing_id },
        );

        let locked_tokens = Option::extract(&mut listed_item.locked_tokens);
        let key = IterableTable::head_key(&locked_tokens);
        while (Option::is_some(&key)) {
            let (val, _, next) = IterableTable::remove_iter(&mut locked_tokens, *Option::borrow(&key));
            Token::deposit_token(sender, val);
            key = next;
        };

        IterableTable::destroy_empty(locked_tokens);

        // claim coins only if someone has bid in the auction, otherwise just remove the auction item
        let current_bidder = Option::get_with_default(&listed_item.current_bidder, seller);
        if (current_bidder != seller) {
            // the auction item can be removed from the auctiondata of the seller once the token and coins are claimed
            let locked_coins = &mut borrow_global_mut<CoinEscrow>(sender_addr).locked_coins;
            // deposit the locked coins to the seller's sender if they have not claimed yet
            if (Table::contains(locked_coins, listing_id)){
                Event::emit_event<ClaimCoinsEvent>(
                    &mut listing_data.claim_coins_events,
                    ClaimCoinsEvent { id: listing_id },
                );
                let coins = Table::remove(locked_coins, listing_id);
                Coin::deposit<TestCoin>(seller, coins);
            };
        };

        let ListedItem {
            min_selling_price: _,
            buy_now_price: _,
            duration: _,
            start_time: _,
            current_bid: _,
            current_bidder: _,
            locked_tokens: locked_tokens,
        } = Table::remove(listed_items, listing_id);

        Option::destroy_none(locked_tokens);
    }

    // claims the escrowed coins from the winning bidder's account to the seller's account
    // called by the seller account
    // if both claims have been made, remove the auction item from the seller's auction data
    public(script) fun claim_coins_script(
        sender: &signer,
        guid_creation_num: u64
    ) acquires CoinEscrow, ListingData {
        let sender_addr = Signer::address_of(sender);

        let listing_id = GUID::create_id(sender_addr, guid_creation_num);
        let listing_data = borrow_global_mut<ListingData>(sender_addr);
        let listed_items = &mut listing_data.listed_items;

        assert!(Table::contains(listed_items, listing_id), ELISTING_INACTIVE);

        let listed_item = Table::borrow(listed_items, listing_id);
        assert!(is_auction_complete(listed_item.start_time, listed_item.duration), EAUCTION_NOT_COMPLETE);
        
        let current_bidder = Option::get_with_default(&listed_item.current_bidder, sender_addr);

        // if no one bid on the auction, no coins to claim
        assert!(current_bidder != sender_addr, ENOT_CLAIMABLE);

        let locked_coins = &mut borrow_global_mut<CoinEscrow>(current_bidder).locked_coins;
        assert!(Table::contains(locked_coins, listing_id), EALREADY_CLAIMED);

        Event::emit_event<ClaimCoinsEvent>(
            &mut listing_data.claim_coins_events,
            ClaimCoinsEvent { id: listing_id },
        );

        let coins = Table::remove(locked_coins, listing_id);
        Coin::deposit<TestCoin>(sender_addr, coins);

        // if the locked token from sender_addr doesn't contain the key listing_id, the highest bidder has already claimed the token
        // and the auction item can be removed from the auctiondata of the seller

        if (Option::is_none(&listed_item.locked_tokens)) {
            let ListedItem {
                min_selling_price: _,
                buy_now_price: _,
                duration: _,
                start_time: _,
                current_bid: _,
                current_bidder: _,
                locked_tokens: locked_tokens,
            } = Table::remove(listed_items, listing_id);

            Option::destroy_none(locked_tokens);
        };
    }

    #[test_only]
    use AptosFramework::TestCoin;

    // Tests the end to end flow of an auction
    #[test(core_framework = @0x1, core_resources = @CoreResources, seller = @0x123, bidder = @0x234)]
    public(script) fun end_to_end_first_claim_coins(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        bidder: signer,
    ) acquires ListingData, CoinEscrow {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let bidder_addr = Signer::address_of(&bidder);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let coins_minted = Coin::mint<TestCoin>(1000, &mint_cap);
        Coin::register<TestCoin>(&bidder);
        Coin::register<TestCoin>(&seller);
        Coin::deposit(bidder_addr, coins_minted);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        bid_script(&bidder, seller_addr, 120, guid_creation_num);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder_addr).locked_coins) == 1, 2);

        Timestamp::update_global_time_for_test(11000001);

        claim_coins_script(&seller, guid_creation_num);
        claim_token_script(&bidder, seller_addr, guid_creation_num);

        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 0, 3);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder_addr).locked_coins) == 0, 4);
        assert!(Token::balance_of(seller_addr, _token_id) == 0, 5);
        assert!(Token::balance_of(bidder_addr, _token_id) == 1, 6);
    }

    // end to end test with multiple bidders
    #[test(
        core_framework = @0x1,
        core_resources = @CoreResources,
        seller = @0x123,
        bidder1 = @0x234,
        bidder2 = @0x235,
        bidder3 = @0x236
    )]
    public(script) fun end_to_end_multiple_bids(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        bidder1: signer,
        bidder2: signer,
        bidder3: signer,
    ) acquires ListingData, CoinEscrow {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let bidder1_addr = Signer::address_of(&bidder1);
        let bidder2_addr = Signer::address_of(&bidder2);
        let bidder3_addr = Signer::address_of(&bidder3);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        Coin::register<TestCoin>(&bidder1);
        Coin::register<TestCoin>(&bidder2);
        Coin::register<TestCoin>(&bidder3);
        Coin::register<TestCoin>(&seller);

        let coins_minted1 = Coin::mint<TestCoin>(5000, &mint_cap);
        Coin::deposit(bidder1_addr, coins_minted1);
        let coins_minted2 = Coin::mint<TestCoin>(5000, &mint_cap);
        Coin::deposit(bidder2_addr, coins_minted2);
        let coins_minted3 = Coin::mint<TestCoin>(5000, &mint_cap);
        Coin::deposit(bidder3_addr, coins_minted3);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        let listing_id = GUID::create_id(seller_addr, guid_creation_num);

        bid_script(&bidder1, seller_addr, 120, guid_creation_num);
        bid_script(&bidder2, seller_addr, 300, guid_creation_num);

        assert!(*Option::borrow(&Table::borrow(&borrow_global<ListingData>(seller_addr).listed_items, listing_id).current_bidder) == bidder2_addr, 2);

        bid_script(&bidder3, seller_addr, 400, guid_creation_num);

        assert!(Table::length(&borrow_global<CoinEscrow>(bidder1_addr).locked_coins) == 0, 3);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder2_addr).locked_coins) == 0, 3);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder3_addr).locked_coins) == 1, 3);

        Timestamp::update_global_time_for_test(11000001);

        claim_coins_script(&seller, guid_creation_num);
        claim_token_script(&bidder3, seller_addr, guid_creation_num);

        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 0, 4);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder1_addr).locked_coins) == 0, 5);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder2_addr).locked_coins) == 0, 6);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder3_addr).locked_coins) == 0, 7);
        assert!(Token::balance_of(seller_addr, _token_id) == 0, 8);
        assert!(Token::balance_of(bidder3_addr, _token_id) == 1, 11);
    }

    // Claims tokens before claiming coins
    // Should result in an error because claim tokens has already 
    // removed the listed item from listing data
    #[test(core_framework = @0x1, core_resources = @CoreResources, seller = @0x123, bidder = @0x234)]
    #[expected_failure]
    public(script) fun claim_tokens_then_claim_coins(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        bidder: signer,
    ) acquires ListingData, CoinEscrow {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let bidder_addr = Signer::address_of(&bidder);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let coins_minted = Coin::mint<TestCoin>(1000, &mint_cap);
        Coin::register<TestCoin>(&bidder);
        Coin::register<TestCoin>(&seller);
        Coin::deposit(bidder_addr, coins_minted);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        bid_script(&bidder, seller_addr, 120, guid_creation_num);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder_addr).locked_coins) == 1, 2);

        Timestamp::update_global_time_for_test(11000001);

        claim_token_script(&bidder, seller_addr, guid_creation_num);
        claim_coins_script(&seller, guid_creation_num);
    }

    // Auction an item twice
    // Should result in an error
    #[test(seller = @0x123)]
    #[expected_failure]
    public(script) fun auction_same_item_twice(
        seller: signer
    ) acquires ListingData {
        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, seller = @0x123, bidder = @0x234)]
    public(script) fun end_to_end_claim_tokens(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        bidder: signer,
    ) acquires ListingData, CoinEscrow {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let bidder_addr = Signer::address_of(&bidder);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let coins_minted = Coin::mint<TestCoin>(1000, &mint_cap);
        Coin::register<TestCoin>(&bidder);
        Coin::register<TestCoin>(&seller);
        Coin::deposit(bidder_addr, coins_minted);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        bid_script(&bidder, seller_addr, 120, guid_creation_num);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder_addr).locked_coins) == 1, 2);

        Timestamp::update_global_time_for_test(11000001);

        claim_token_script(&bidder, seller_addr, guid_creation_num);
        
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 0, 3);
        assert!(Table::length(&borrow_global<CoinEscrow>(bidder_addr).locked_coins) == 0, 4);
        assert!(Token::balance_of(seller_addr, _token_id) == 0, 5);
        assert!(Token::balance_of(bidder_addr, _token_id) == 1, 6);
    }

    #[test(core_framework = @0x1, core_resources = @CoreResources, seller = @0x123, bidder = @0x234)]
    public(script) fun buy_now_auction(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        bidder: signer,
    ) acquires ListingData, CoinEscrow {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let bidder_addr = Signer::address_of(&bidder);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let coins_minted = Coin::mint<TestCoin>(1000, &mint_cap);
        Coin::register<TestCoin>(&bidder);
        Coin::register<TestCoin>(&seller);
        Coin::deposit(bidder_addr, coins_minted);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 500, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        bid_script(&bidder, seller_addr, 500, guid_creation_num);
        
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 0, 3);
        assert!(!exists<CoinEscrow>(bidder_addr), 4);
        assert!(Token::balance_of(seller_addr, _token_id) == 0, 5);
        assert!(Token::balance_of(bidder_addr, _token_id) == 1, 6);
    }

    // Tests the end to end flow of an auction
    #[test(core_framework = @0x1, core_resources = @CoreResources, seller = @0x123, buyer = @0x234)]
    public(script) fun end_to_end_fps(
        core_framework: signer,
        core_resources: signer,
        seller: signer,
        buyer: signer,
    ) acquires ListingData {
        Timestamp::set_time_has_started_for_testing(&core_resources);

        let _collection_name = b"CollectionName";
        let _token_name = b"TokenName";
        let _token_id = create_token(&seller, _collection_name, _token_name);

        let seller_addr = Signer::address_of(&seller);
        let buyer_addr = Signer::address_of(&buyer);

        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let coins_minted = Coin::mint<TestCoin>(1000, &mint_cap);
        Coin::register<TestCoin>(&buyer);
        Coin::register<TestCoin>(&seller);
        Coin::deposit(buyer_addr, coins_minted);

        Coin::destroy_burn_cap<TestCoin>(burn_cap);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);

        let creators = Vector::empty<address>();
        let collection_names = Vector::empty<vector<u8>>();
        let token_names = Vector::empty<vector<u8>>();

        Vector::push_back<address>(&mut creators, seller_addr);
        Vector::push_back<vector<u8>>(&mut collection_names, _collection_name);
        Vector::push_back<vector<u8>>(&mut token_names, _token_name);

        Timestamp::update_global_time_for_test(1000000);

        list_token_script(&seller, creators, collection_names, token_names, 100, 100, 10000000);
        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 1, 1);

        let guid_creation_num = GUID::get_next_creation_num(seller_addr) - 1;
        buy_token_script(&buyer, seller_addr, guid_creation_num);

        assert!(Table::length(&borrow_global<ListingData>(seller_addr).listed_items) == 0, 3);
        assert!(!exists<CoinEscrow>(buyer_addr), 4);
        assert!(Token::balance_of(seller_addr, _token_id) == 0, 5);
        assert!(Token::balance_of(buyer_addr, _token_id) == 1, 6);
    }

    fun create_token(creator: &signer, collection_name: vector<u8>, token_name: vector<u8>): TokenId {
        use Std::ASCII;
        use Std::Option;

        Token::create_collection(
            creator,
            ASCII::string(collection_name),
            ASCII::string(b"Collection: Hello, World"),
            ASCII::string(b"https://aptos.dev"),
            Option::some(1),
        );

        Token::create_token(
            creator,
            ASCII::string(collection_name),
            ASCII::string(token_name),
            ASCII::string(b"Hello, Token"),
            false,
            1,
            Option::none(),
            ASCII::string(b"https://aptos.dev"),
            0
        )
    }
}
